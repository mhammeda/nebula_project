use {
    crate::{
        fed::{client::Client, PostFilters},
        models::{
            database,
            fed::PostEdit,
            internal::{NewPost, Post, UserId},
        },
        AppData, Error,
    },
    actix_identity::Identity,
    actix_web::{delete, get, post, put, web, HttpResponse, Responder, Result},
    futures::future::join_all,
    log::error,
    sqlx::{Pool, Postgres},
    std::{
        collections::HashMap,
        convert::{TryFrom, TryInto},
    },
    uuid::Uuid,
};

/// Recursively fetches all child posts of the supplied root post
async fn fetch_children(root: Uuid, executor: &Pool<Postgres>) -> anyhow::Result<Vec<Post>, Error> {
    let mut map = HashMap::<Uuid, Vec<Post>>::new();

    // Retrieve all posts from the database iteratively
    {
        let mut stack = vec![root];
        while !stack.is_empty() {
            // safe to unwrap as it is preceded by a check that it is not empty
            let parent = stack.pop().unwrap();

            let posts: Vec<Post> = sqlx::query_as!(
                database::Post,
                r#"
                    SELECT * FROM posts
                    WHERE parent = $1
                "#,
                parent
            )
            .fetch_all(executor)
            .await?
            .into_iter()
            .map(|row| row.try_into())
            .collect::<Result<_, _>>()?;

            for post in &posts {
                stack.push(post.id);
            }
            map.insert(parent, posts);
        }
    }

    // Recursively build post tree
    let posts = {
        fn get_children(parent: Uuid, map: &mut HashMap<Uuid, Vec<Post>>) -> Vec<Post> {
            match map.remove(&parent) {
                Some(mut posts) => {
                    for post in &mut posts {
                        post.children = get_children(post.id, map);
                    }

                    return posts;
                }
                None => return vec![],
            }
        }

        let mut posts = vec![];

        posts.append(map.get_mut(&root).unwrap());
        for post in &mut posts {
            post.children = get_children(post.id, &mut map);
        }

        posts
    };

    Ok(posts)
}

/// Get post
#[get("/internal/posts/{id}")]
pub(crate) async fn get_post(
    data: web::Data<AppData>,
    web::Path(post_id): web::Path<Uuid>,
) -> Result<impl Responder, Error> {
    // fetch post from database
    let mut post: Post = sqlx::query_as!(
        database::Post,
        r#"
            SELECT * FROM posts
            WHERE id = $1
        "#,
        post_id
    )
    .fetch_one(&data.pool)
    .await?
    .try_into()?;

    post.children = match fetch_children(post.id, &data.pool).await {
        Ok(children) => children,
        Err(e) => {
            error!("Error occured whilst executing query: {}", e);
            return Ok(HttpResponse::InternalServerError().finish());
        }
    };

    // Return a successful response containing the Community
    Ok(HttpResponse::Ok().json(post))
}

/// Get bulk posts
#[get("/internal/posts")]
pub(crate) async fn get_bulk_post(
    identity: Identity,
    data: web::Data<AppData>,
) -> Result<impl Responder, Error> {
    let username = match identity.identity() {
        Some(s) => s,
        None => return Ok(HttpResponse::Unauthorized().into()),
    };

    let subscriptions = sqlx::query!(
        r#"
            SELECT community FROM subscriptions
            WHERE username = $1
            AND host = $2
        "#,
        username,
        crate::host!()
    )
    .fetch_all(&data.pool)
    .await?
    .into_iter()
    .map(|x| x.community)
    .collect::<Vec<_>>();

    // if user is not subscribed, show content from everywhere
    if subscriptions.is_empty() {
        // Fetch local posts
        let mut posts: Vec<Post> = sqlx::query_as!(
            database::Post,
            r#"
                SELECT * FROM posts
                WHERE parent IS NULL
            "#,
        )
        .fetch_all(&data.pool)
        .await?
        .into_iter()
        .map(|x| x.try_into())
        .collect::<Result<_, _>>()?;

        // Fetch remote posts
        posts.append(
            &mut join_all(
                sqlx::query!(
                    r#"
                        SELECT host FROM remotes
                    "#,
                )
                .fetch_all(&data.pool)
                .await?
                .into_iter()
                .map(|remote| {
                    let client = Client::new(&data.privkey);
                    let username = username.clone();
                    Box::pin(async move {
                        match client
                            .get_posts(
                                &remote.host,
                                PostFilters {
                                    limit: Some(5),
                                    ..PostFilters::default()
                                },
                                &username,
                            )
                            .await
                        {
                            Ok(posts) => Some(
                                posts
                                    .into_iter()
                                    .map(|p| Post {
                                        id: p.id,
                                        host: remote.host.clone(),
                                        community: p.community,
                                        parent_post: p.parent_post,
                                        children: vec![],
                                        title: p.title,
                                        content: p.content,
                                        author: p.author.into(),
                                        modified: p.modified,
                                        created: p.created,
                                    })
                                    .collect::<Vec<_>>(),
                            ),
                            Err(e) => {
                                error!(
                                    "Error occured while fetching posts from remote {}: {}",
                                    remote.host, e
                                );
                                None
                            }
                        }
                    })
                }),
            )
            .await
            .into_iter()
            .filter_map(|x| x)
            .flatten()
            .collect(),
        );

        // Return a successful response containing the IDs in JSON
        Ok(HttpResponse::Ok().json(posts))
    } else
    // if user is subscribed to communities, show only content from those communities
    {
        let posts: Vec<Post> = join_all(subscriptions.into_iter().map(|community| {
            let pool = data.pool.clone();
            Box::pin(async move {
                let query = sqlx::query_as!(
                    database::Post,
                    r#"
                            SELECT * FROM posts
                            WHERE parent IS NULL
                            AND community = $1
                        "#,
                    community
                )
                .fetch_all(&pool)
                .await;

                match query {
                    Ok(xs) => {
                        let posts = xs
                            .into_iter()
                            .map(|x| Post::try_from(x))
                            .collect::<Result<Vec<_>, _>>();

                        match posts {
                            Ok(xs) => {
                                xs
                            },
                            Err(e) => {
                                error!("Error returned while converting database posts to internal posts: {}", e);
                                vec![]
                            }
                        }
                    }
                    Err(e) => {
                        error!("Error returned while fetching posts database: {}", e);
                        vec![]
                    }
                }
            })
        }))
        .await
        .into_iter()
        .flatten()
        .collect();

        // Return a successful response containing the IDs in JSON
        Ok(HttpResponse::Ok().json(posts))
    }
}

/// Create new post
#[post("/internal/posts")]
pub(crate) async fn create_post(
    identity: Identity,
    data: web::Data<AppData>,
    web::Json(body): web::Json<NewPost>,
) -> Result<impl Responder, Error> {
    let username = match identity.identity() {
        Some(s) => s,
        None => return Ok(HttpResponse::Unauthorized().into()),
    };

    let now = chrono::Local::now().timestamp();
    let p = database::Post {
        id: Uuid::new_v4(),
        community: body.community,
        parent: body.parent_post,
        author_username: username.clone(),
        author_host: crate::host!(),
        title: body.title,
        content: serde_json::to_value(&body.content)?,
        created: now,
        modified: now,
    };

    sqlx::query!(
        r#"
            INSERT INTO posts
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
        p.id,
        p.community,
        p.parent,
        p.author_username,
        p.author_host,
        p.title,
        p.content,
        p.created,
        p.modified,
    )
    .execute(&data.pool)
    .await?;

    Ok(HttpResponse::Ok().json(Post::try_from(p)?))
}

/// Edit a post
#[put("/internal/posts/{id}")]
pub(crate) async fn edit_post(
    identity: Identity,
    data: web::Data<AppData>,
    web::Json(body): web::Json<PostEdit>,
    web::Path(id): web::Path<Uuid>,
) -> Result<impl Responder, Error> {
    let username = match identity.identity() {
        Some(s) => s,
        None => return Ok(HttpResponse::Unauthorized().into()),
    };

    // Execute query
    sqlx::query!(
        r#"
            UPDATE posts
            SET content = $1, title = $2, modified = $3
            FROM users
            WHERE posts.id = $4
            AND users.username = $5
            AND users.host = $6
        "#,
        serde_json::to_value(body.content)?,
        body.title,
        chrono::Local::now().timestamp(),
        id,
        username,
        crate::host!()
    )
    .execute(&data.pool)
    .await?;

    // fetch post from database
    let mut post: Post = sqlx::query_as!(
        database::Post,
        r#"
            SELECT * FROM posts
            WHERE id = $1
        "#,
        id
    )
    .fetch_one(&data.pool)
    .await?
    .try_into()?;

    post.children = match fetch_children(post.id, &data.pool).await {
        Ok(children) => children,
        Err(e) => {
            error!("Error occured whilst executing query: {}", e);
            return Ok(HttpResponse::InternalServerError().finish());
        }
    };

    // Return a successful response containing the edited post
    Ok(HttpResponse::Ok().json(post))
}

/// Delete post
#[delete("/internal/posts/{id}")]
pub(crate) async fn delete_post(
    identity: Identity,
    data: web::Data<AppData>,
    web::Path(post_id): web::Path<Uuid>,
) -> Result<impl Responder, Error> {
    let user = match identity.identity() {
        Some(s) => UserId {
            username: s,
            host: crate::host!(),
        },
        None => {
            // must be logged in to delete a community
            return Ok(HttpResponse::Unauthorized());
        }
    };

    let author = {
        let row = sqlx::query!(
            r#"
                SELECT author_username, author_host FROM posts
                WHERE id = $1
            "#,
            post_id,
        )
        .fetch_one(&data.pool)
        .await?;

        UserId {
            username: row.author_username,
            host: row.author_host,
        }
    };

    // must be the author to delete a post
    if user != author {
        return Ok(HttpResponse::Unauthorized());
    }

    sqlx::query!(
        r#"
            DELETE FROM posts
            WHERE id = $1
        "#,
        post_id
    )
    .execute(&data.pool)
    .await?;

    Ok(HttpResponse::Ok())
}

/// Fuzzy string search by title and post content
#[get("/internal/posts/search/{search}")]
pub(crate) async fn search_posts(
    data: web::Data<AppData>,
    web::Path(search): web::Path<String>,
) -> Result<impl Responder, Error> {
    let posts: Vec<Post> = sqlx::query_as!(
        database::Post,
        r#"
            SELECT * FROM posts
            WHERE title % $1
            OR content::TEXT % $1
        "#,
        search
    )
    .fetch_all(&data.pool)
    .await?
    .into_iter()
    .map(|x| x.try_into())
    .collect::<Result<_, _>>()?;

    Ok(HttpResponse::Ok().json(posts))
}

#[cfg(test)]
mod test {
    use {
        crate::{
            models::{
                database::{PostContent, TextContent},
                internal::Post,
            },
            test::{new_user_login, ADDR},
        },
        actix_web::http::{header::CONTENT_TYPE, StatusCode},
    };

    #[actix_rt::test]
    async fn create_post_success() {
        let (client, username, cookie) = new_user_login().await;

        let res = client
            .post(format!("{}/internal/communities", *ADDR))
            .header(CONTENT_TYPE, "application/json")
            .cookie(cookie.clone())
            .send_body(
                r#"
                    {
                        "id": "community2",
                        "title": "My Third Community!",
                        "description": "My community description"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let mut res = client
            .post(&format!("{}/internal/posts", *ADDR))
            .header(CONTENT_TYPE, "application/json")
            .cookie(cookie.clone())
            .send_body(
                r#"
                    {
                        "community": "community2",
                        "title": "My New Post!",
                        "content": [
                            {
                                "text": {
                                    "text": "Post content goes here!"
                                }
                            }
                        ]
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let post: Post = res.json().await.unwrap();
        assert_eq!(post.author.username, username);

        let mut res = client
            .get(&format!("{}/internal/posts/{}", *ADDR, post.id))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.json::<Post>().await.unwrap(), post);
    }

    #[actix_rt::test]
    async fn delete_post_success() {
        let (client, username, cookie) = new_user_login().await;

        let res = client
            .post(&format!("{}/internal/communities", *ADDR))
            .header(CONTENT_TYPE, "application/json")
            .cookie(cookie.clone())
            .send_body(
                r#"
                    {
                        "id": "community3",
                        "title": "My Community!",
                        "description": "My community description"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let mut res = client
            .post(&format!("{}/internal/posts", *ADDR))
            .header(CONTENT_TYPE, "application/json")
            .cookie(cookie.clone())
            .send_body(
                r#"
                    {
                        "community": "community3",
                        "title": "Delete Me!",
                        "content": [
                            {
                                "text": {
                                    "text": "Post content goes here!"
                                }
                            }
                        ]
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let post: Post = res.json().await.unwrap();
        assert_eq!(post.author.username, username);

        let res = client
            .delete(&format!("{}/internal/posts/{}", *ADDR, post.id))
            .cookie(cookie.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[actix_rt::test]
    async fn comment_success() {
        let (client, username, cookie) = new_user_login().await;

        // Create community to post to
        let res = client
            .post(&format!("{}/internal/communities", *ADDR))
            .header(CONTENT_TYPE, "application/json")
            .cookie(cookie.clone())
            .send_body(
                r#"
                    {
                        "id": "community5",
                        "title": "My Community!",
                        "description": "My community description"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // Create top level post
        let mut res = client
            .post(&format!("{}/internal/posts", *ADDR))
            .header(CONTENT_TYPE, "application/json")
            .cookie(cookie.clone())
            .send_body(
                r#"
                    {
                        "community": "community5",
                        "title": "Top Level Post!",
                        "content": [
                            {
                                "text": {
                                    "text": "Top level post goes here!"
                                }
                            }
                        ]
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let post: Post = res.json().await.unwrap();
        assert_eq!(post.author.username, username);

        // Check it is correct
        let mut res = client
            .get(&format!("{}/internal/posts/{}", *ADDR, post.id))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.json::<Post>().await.unwrap(), post);

        // Create child post
        let mut res = client
            .post(&format!("{}/internal/posts", *ADDR))
            .header(CONTENT_TYPE, "application/json")
            .cookie(cookie.clone())
            .send_body(&format!(
                "{{
                        \"community\": \"community5\",
                        \"parentPost\": \"{}\",
                        \"title\": \"\",
                        \"content\": [
                                {{
                                    \"text\": {{
                                        \"text\": \"This is a comment!\"
                                    }}
                                }}
                            ]
                    }}",
                post.id
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let child: Post = res.json().await.unwrap();
        assert_eq!(child.author.username, username);
        assert_eq!(child.parent_post, Some(post.id));

        // Getting top level post should now contain child
        let mut res = client
            .get(&format!("{}/internal/posts/{}", *ADDR, post.id))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.json::<Post>().await.unwrap().children, vec![child]);
    }

    #[actix_rt::test]
    async fn nested_comment_success() {
        let (client, username, cookie) = new_user_login().await;

        // Create community to post to
        let res = client
            .post(&format!("{}/internal/communities", *ADDR))
            .header(CONTENT_TYPE, "application/json")
            .cookie(cookie.clone())
            .send_body(
                r#"
                    {
                        "id": "community6",
                        "title": "My Community!",
                        "description": "My community description"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // Create top level post
        let mut res = client
            .post(&format!("{}/internal/posts", *ADDR))
            .header(CONTENT_TYPE, "application/json")
            .cookie(cookie.clone())
            .send_body(
                r#"
                    {
                        "community": "community6",
                        "title": "Top Level Post!",
                        "content": [
                            {
                                "text": {
                                    "text": "Top level post goes here!"
                                }
                            }
                        ]
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let post: Post = res.json().await.unwrap();
        assert_eq!(post.author.username, username);

        // Check it is correct
        let mut res = client
            .get(&format!("{}/internal/posts/{}", *ADDR, post.id))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.json::<Post>().await.unwrap(), post);

        // Create child post
        let mut res = client
            .post(&format!("{}/internal/posts", *ADDR))
            .header(CONTENT_TYPE, "application/json")
            .cookie(cookie.clone())
            .send_body(&format!(
                "{{
                    \"community\": \"community6\",
                    \"parentPost\": \"{}\",
                    \"title\": \"Child Post Title (probably ignored!)\",
                    \"content\": [
                            {{
                                \"text\": {{
                                    \"text\": \"Child post content here!\"
                                }}
                            }}
                        ]
                }}",
                post.id
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let mut child: Post = res.json().await.unwrap();
        let child_id = child.id;
        assert_eq!(child.author.username, username);
        assert_eq!(child.parent_post, Some(post.id));

        // Create child to child post
        let mut res = client
            .post(&format!("{}/internal/posts", *ADDR))
            .header(CONTENT_TYPE, "application/json")
            .cookie(cookie.clone())
            .send_body(&format!(
                "{{
                    \"community\": \"community6\",
                    \"parentPost\": \"{}\",
                    \"title\": \"Child to child of post Title (probably ignored!)\",
                    \"content\": [
                        {{
                            \"text\": {{
                                \"text\": \"Grandchild post content here!\"
                            }}
                        }}
                    ]
                }}",
                child.id
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let childchild: Post = res.json().await.unwrap();
        assert_eq!(childchild.author.username, username);
        assert_eq!(childchild.parent_post, Some(child.id));

        // Getting top level post should now contain child
        let mut res = client
            .get(&format!("{}/internal/posts/{}", *ADDR, post.id))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        child.children = vec![childchild.clone()];
        assert_eq!(res.json::<Post>().await.unwrap().children, vec![child]);

        // Getting child post should now contain child of child
        let mut res = client
            .get(&format!("{}/internal/posts/{}", *ADDR, child_id))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.json::<Post>().await.unwrap().children, vec![childchild]);
    }

    #[actix_rt::test]
    async fn edit_post_success() {
        let (client, _, cookie) = new_user_login().await;

        // Create community to post to
        let res = client
            .post(&format!("{}/internal/communities", *ADDR))
            .cookie(cookie.clone())
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "id": "posts::community6",
                        "title": "Edit post community",
                        "description": "My community description"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // Create post
        let mut res = client
            .post(&format!("{}/internal/posts", *ADDR))
            .cookie(cookie.clone())
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "community": "posts::community6",
                        "title": "Top Level Post!",
                        "content": [
                            {
                                "text": {
                                    "text": "Top level post goes here!"
                                }
                            }
                        ]
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let post: Post = res.json().await.unwrap();

        // Check it is correct
        let mut res = client
            .get(&format!("{}/internal/posts/{}", *ADDR, post.id))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.json::<Post>().await.unwrap(), post);

        // Edit post
        let mut res = client
            .put(&format!("{}/internal/posts/{}", *ADDR, post.id))
            .cookie(cookie.clone())
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "title": "Edited title!",
                        "content": [
                            {
                                "text": {
                                    "text": "Edited content goes here!"
                                }
                            }
                        ]
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let post: Post = res.json().await.unwrap();

        // Check it is correct
        let mut res = client
            .get(&format!("{}/internal/posts/{}", *ADDR, post.id))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.json::<Post>().await.unwrap(), post);
        assert_eq!(post.title, "Edited title!".to_owned());
        assert_eq!(
            post.content,
            vec![PostContent::Text(TextContent {
                text: "Edited content goes here!".to_owned()
            })]
        );
    }
}
