use {
    crate::{
        models::{
            database,
            fed::{NewPost, Post, PostEdit, UserId},
        },
        util::{get_client_host, get_user_id},
        AppData, Error,
    },
    actix_web::{delete, get, post, put, web, HttpRequest, HttpResponse, Responder, Result},
    serde::{Deserialize, Serialize},
    std::{borrow::Cow, convert::TryInto},
    uuid::Uuid,
};

/// Filters for GET /fed/posts requests
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PostFilters {
    pub limit: Option<i64>,
    pub community: Option<String>,
    pub min_date: Option<i64>,
    pub parent_post: Option<Uuid>,
    pub include_sub_children_posts: Option<bool>,
    pub content_type: Option<String>,
}

/// Gets all posts
#[get("/fed/posts")]
pub(crate) async fn get_posts(
    req: HttpRequest,
    data: web::Data<AppData>,
    web::Query(filters): web::Query<PostFilters>,
) -> Result<impl Responder, Error> {
    get_user_id(&req)?;
    get_client_host(&req)?;

    // Execute query
    let mut posts: Vec<Post> = sqlx::query_as!(
        database::Post,
        r#"
            SELECT * FROM posts
            WHERE ($2::VARCHAR is null OR community = $2)
            AND ($3::BIGINT is null OR created >= $3)
            AND ($4::UUID is null OR parent = $4)
            AND ($5::VARCHAR is null OR
                EXISTS (SELECT 1 FROM jsonb_array_elements(content) WHERE value ? $5))
            ORDER BY modified DESC
            LIMIT $1
        "#,
        filters.limit,
        filters.community,
        filters.min_date,
        filters.parent_post,
        filters.content_type
    )
    .fetch_all(&data.pool)
    .await?
    .into_iter()
    .map(|r| r.try_into())
    .collect::<Result<_, _>>()?;

    // Populate children field for each post
    for post in &mut posts {
        let children: Vec<Uuid> = sqlx::query!(
            r#"
                SELECT id FROM posts
                WHERE parent = $1
            "#,
            post.id
        )
        .fetch_all(&data.pool)
        .await?
        .into_iter()
        .map(|x| x.id)
        .collect();

        post.children = children;
    }

    // Fetch children
    if filters.include_sub_children_posts == Some(true) {
        unimplemented!();
    }

    // Return a successful response containing the posts in JSON
    Ok(HttpResponse::Ok().json(posts))
}

/// Creates a new post
#[post("/fed/posts")]
pub(crate) async fn create_post(
    req: HttpRequest,
    data: web::Data<AppData>,
    web::Json(body): web::Json<NewPost>,
) -> Result<impl Responder, Error> {
    let author = UserId {
        id: get_user_id(&req)?.to_owned(),
        host: get_client_host(&req)?.to_owned(),
    };

    // ensure that author exists
    match sqlx::query!(
        r#"
            INSERT INTO users VALUES ($1, $2)
        "#,
        &author.id,
        &author.host
    )
    .execute(&data.pool)
    .await
    {
        Ok(_) => {}
        Err(sqlx::Error::Database(e)) => {
            if e.code() != Some(Cow::from("23505")) {
                return Err(sqlx::Error::Database(e).into());
            }
        }
        Err(e) => return Err(e.into()),
    };

    let now = chrono::Local::now().timestamp();

    let p = Post {
        id: Uuid::new_v4(),
        community: body.community,
        parent_post: body.parent_post,
        children: vec![],
        author,
        title: match body.title {
            Some(s) => s,
            None => "".to_owned(),
        },
        content: body.content,
        created: now,
        modified: now,
    };

    // Execute query
    sqlx::query!(
        r#"
            INSERT INTO posts VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
        p.id,
        p.community,
        p.parent_post,
        p.author.id,
        p.author.host,
        p.title,
        serde_json::to_value(&p.content)?,
        p.created,
        p.modified,
    )
    .execute(&data.pool)
    .await?;

    Ok(HttpResponse::Ok().json(p))
}

/// Gets a post by ID
#[get("/fed/posts/{id}")]
pub(crate) async fn get_post_by_id(
    req: HttpRequest,
    data: web::Data<AppData>,
    web::Path(id): web::Path<Uuid>,
) -> Result<impl Responder, Error> {
    get_user_id(&req)?;
    get_client_host(&req)?;

    // Execute query
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

    // Populate children field
    post.children = sqlx::query!(
        r#"
            SELECT id FROM posts
            WHERE parent = $1
        "#,
        post.id
    )
    .fetch_all(&data.pool)
    .await?
    .into_iter()
    .map(|x| x.id)
    .collect();

    // Return a successful response containing the Post
    Ok(HttpResponse::Ok().json(post))
}

/// Edit a post
#[put("/fed/posts/{id}")]
pub(crate) async fn edit_post(
    req: HttpRequest,
    data: web::Data<AppData>,
    web::Path(id): web::Path<Uuid>,
    web::Json(body): web::Json<PostEdit>,
) -> Result<impl Responder, Error> {
    let username = get_user_id(&req)?;
    let host = get_client_host(&req)?;

    let now = chrono::Local::now().timestamp();

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
        now,
        id,
        username,
        host
    )
    .execute(&data.pool)
    .await?;

    Ok(HttpResponse::Ok())
}

/// Deletes a post
#[delete("/fed/posts/{id}")]
pub(crate) async fn delete_post(
    req: HttpRequest,
    data: web::Data<AppData>,
    web::Path(id): web::Path<Uuid>,
) -> Result<impl Responder, Error> {
    let username = get_user_id(&req)?;
    let host = get_client_host(&req)?;

    sqlx::query!(
        r#"
            DELETE FROM posts
            USING users
            WHERE posts.id = $1
            AND users.username = $2
            AND users.host = $3
        "#,
        id,
        username,
        host
    )
    .execute(&data.pool)
    .await?;

    Ok(HttpResponse::Ok())
}
