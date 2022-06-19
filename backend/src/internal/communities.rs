use {
    crate::{
        models::{
            database,
            internal::{Community, NewCommunity, UserId},
        },
        util::{is_moderator, user_exists},
        AppData, Error,
    },
    actix_identity::Identity,
    actix_web::{delete, get, post, web, HttpResponse, Responder, Result},
    futures::future::join_all,
    log::error,
};

/// Create new community
#[post("/internal/communities")]
pub(crate) async fn create_community(
    identity: Identity,
    data: web::Data<AppData>,
    web::Json(body): web::Json<NewCommunity>,
) -> Result<impl Responder, Error> {
    let username = match identity.identity() {
        Some(s) => s,
        None => {
            // must be logged in to create community
            return Ok(HttpResponse::Unauthorized().into());
        }
    };

    let c = Community {
        id: body.id,
        host: crate::host!(),
        title: body.title,
        description: body.description,
        moderators: vec![UserId {
            username: username.clone(),
            host: crate::host!(),
        }],
        created: chrono::Local::now().timestamp(),
    };

    // insert community into database
    sqlx::query!(
        r#"
            INSERT INTO communities VALUES ($1, $2, $3, $4)
        "#,
        c.id,
        c.title,
        c.description,
        c.created
    )
    .execute(&data.pool)
    .await?;

    // add creator as moderator
    sqlx::query!(
        r#"
            INSERT INTO moderators VALUES ($1, $2, $3)
        "#,
        username,
        crate::host!(),
        c.id,
    )
    .execute(&data.pool)
    .await?;

    // subscribe creator to community
    sqlx::query!(
        r#"
            INSERT INTO subscriptions VALUES ($1, $2, $3)
        "#,
        username,
        crate::host!(),
        c.id,
    )
    .execute(&data.pool)
    .await?;

    Ok(HttpResponse::Ok().json(c))
}

/// Gets all communities
#[get("/internal/communities")]
pub(crate) async fn get_communities(data: web::Data<AppData>) -> Result<impl Responder, Error> {
    let mut communities: Vec<Community> = sqlx::query_as!(
        database::Community,
        r#"
            SELECT * FROM communities
        "#,
    )
    .fetch_all(&data.pool)
    .await?
    .into_iter()
    .map(|r| Community {
        id: r.id,
        host: crate::host!(),
        title: r.title,
        description: r.description,
        moderators: vec![],
        created: r.created,
    })
    .collect();

    for community in &mut communities {
        community.moderators = sqlx::query!(
            r#"
                SELECT username, host FROM moderators
                WHERE community = $1
            "#,
            community.id
        )
        .fetch_all(&data.pool)
        .await?
        .into_iter()
        .map(|r| UserId {
            username: r.username,
            host: r.host,
        })
        .collect();
    }

    // get remote communities
    let mut remote_communities = {
        let remotes: Vec<String> = sqlx::query!(
            r#"
                SELECT host FROM remotes
            "#,
        )
        .fetch_all(&data.pool)
        .await?
        .into_iter()
        .map(|r| r.host)
        .collect();

        join_all(remotes.into_iter().map(|remote| {
            let privkey = data.privkey.clone();
            let client = crate::Client::new(&privkey);

            Box::pin(async move {
                match client.get_communities(&remote).await {
                    Ok(ids) => {
                        join_all(ids.into_iter().map(|community_id| {

                            let remote = remote.clone();
                            let client =  crate::Client::new(&privkey);

                            Box::pin(async move {
                                match client.get_community(&remote, &community_id).await {
                                    Ok(c) => {
                                        Some(Community {
                                            id: c.id.clone(),
                                            host: remote.clone(),
                                            title: c.title,
                                            description: c.description,
                                            moderators: c.admins.into_iter().map(|u| UserId::from(u)).collect(),
                                            created: 0,
                                        })
                                    },
                                    Err(e) => {
                                        error!(
                                            "Error returned while fetching community \"{}\" from {}: {}",
                                            community_id, &remote, e
                                        );
                                        None
                                    }
                                }
                            })
                        }))
                        .await
                    }
                    Err(e) => {
                        error!(
                            "Error returned while fetching community IDs from {}: {}",
                            &remote, e
                        );
                        vec![]
                    }
                }
            })
        }))
        .await.into_iter().flatten().filter_map(|x| x).collect()
    };
    communities.append(&mut remote_communities);

    // Return a successful response containing the IDs in JSON
    Ok(HttpResponse::Ok().json(communities))
}

/// Gets a community by ID
#[get("/internal/communities/{id}")]
pub(crate) async fn get_community_by_id(
    data: web::Data<AppData>,
    web::Path(community_id): web::Path<String>,
) -> Result<impl Responder, Error> {
    // Fetch community
    let row = sqlx::query_as!(
        database::Community,
        r#"
            SELECT * FROM communities
            WHERE id = $1
        "#,
        community_id
    )
    .fetch_one(&data.pool)
    .await?;

    let mut community = Community {
        id: row.id.clone(),
        host: crate::host!(),
        title: row.title,
        description: row.description,
        moderators: vec![],
        created: row.created,
    };

    // fetch moderators
    community.moderators = sqlx::query!(
        r#"
            SELECT username, host FROM moderators
            WHERE community = $1
        "#,
        community.id
    )
    .fetch_all(&data.pool)
    .await?
    .into_iter()
    .map(|r| UserId {
        username: r.username,
        host: r.host,
    })
    .collect();

    // Return a successful response containing the Community
    Ok(HttpResponse::Ok().json(community))
}

/// Delete community
#[delete("/internal/communities/{id}")]
pub(crate) async fn delete_community(
    identity: Identity,
    data: web::Data<AppData>,
    web::Path(community): web::Path<String>,
) -> Result<impl Responder, Error> {
    let username = match identity.identity() {
        Some(s) => s,
        None => {
            // must be logged in to delete a community
            return Ok(HttpResponse::Unauthorized());
        }
    };

    // must be a moderator to delete a community
    if !is_moderator(username, crate::host!(), &community, &data.pool).await? {
        return Ok(HttpResponse::Unauthorized());
    }

    sqlx::query!(
        r#"
            DELETE FROM communities
            WHERE id = $1
        "#,
        community,
    )
    .execute(&data.pool)
    .await?;

    Ok(HttpResponse::Ok())
}

/// Subscribes the current user to community with the supplied ID
#[post("/internal/communities/{id}/subscribe")]
pub(crate) async fn subscribe_community(
    identity: Identity,
    data: web::Data<AppData>,
    web::Path(community): web::Path<String>,
) -> Result<impl Responder, Error> {
    let username = match identity.identity() {
        Some(s) => s,
        None => {
            // must be logged in to subscribe to a community
            return Ok(HttpResponse::Unauthorized());
        }
    };

    sqlx::query!(
        r#"
            INSERT INTO subscriptions VALUES ($1, $2, $3)
            ON CONFLICT DO NOTHING
        "#,
        username,
        crate::host!(),
        community
    )
    .execute(&data.pool)
    .await?;

    Ok(HttpResponse::Ok())
}

/// Unsubscribes the current user to community with the supplied ID
#[delete("/internal/communities/{id}/subscribe")]
pub(crate) async fn unsubscribe_community(
    identity: Identity,
    data: web::Data<AppData>,
    web::Path(community): web::Path<String>,
) -> Result<impl Responder, Error> {
    let username = match identity.identity() {
        Some(s) => s,
        None => {
            // must be logged in to subscribe to a community
            return Ok(HttpResponse::Unauthorized());
        }
    };

    sqlx::query!(
        r#"
            DELETE FROM subscriptions
            WHERE username = $1
            AND host = $2
            AND community = $3
        "#,
        username,
        crate::host!(),
        community
    )
    .execute(&data.pool)
    .await?;

    Ok(HttpResponse::Ok())
}

/// Fuzzy string search by community title and description
#[get("/internal/communities/search/{search}")]
pub(crate) async fn search_communities(
    data: web::Data<AppData>,
    web::Path(search): web::Path<String>,
) -> Result<impl Responder, Error> {
    // Execute query
    let mut communities: Vec<Community> = sqlx::query_as!(
        database::Community,
        r#"
            SELECT * FROM communities
            WHERE title % $1
            OR description % $1
        "#,
        search
    )
    .fetch_all(&data.pool)
    .await?
    .into_iter()
    .map(|r| Community {
        id: r.id,
        host: crate::host!(),
        title: r.title,
        description: r.description,
        moderators: vec![],
        created: r.created,
    })
    .collect();

    for community in &mut communities {
        community.moderators = sqlx::query!(
            r#"
                SELECT username, host FROM moderators
                WHERE community = $1
            "#,
            community.id
        )
        .fetch_all(&data.pool)
        .await?
        .into_iter()
        .map(|r| UserId {
            username: r.username,
            host: r.host,
        })
        .collect();
    }

    Ok(HttpResponse::Ok().json(communities))
}

/// Adds a moderator to a community
#[post("/internal/communities/{community}/moderators/{user}")]
pub(crate) async fn add_community_moderator(
    identity: Identity,
    data: web::Data<AppData>,
    web::Path((community, user)): web::Path<(String, String)>,
) -> Result<impl Responder, Error> {
    // exit early if requesting user is not authorised
    let requesting_user = match identity.identity() {
        Some(s) => s,
        None => {
            // must be logged in to subscribe to a community
            return Ok(HttpResponse::Unauthorized());
        }
    };

    // requesting user must be a moderator to add new moderator
    if !is_moderator(requesting_user, crate::host!(), &community, &data.pool).await? {
        return Ok(HttpResponse::Unauthorized());
    }

    // target user must exist
    if !user_exists(&user, crate::host!(), &data.pool).await? {
        return Ok(HttpResponse::NotFound());
    }

    // add new user to moderators
    sqlx::query!(
        r#"
            INSERT INTO moderators VALUES ($1, $2, $3)
            ON CONFLICT DO NOTHING
        "#,
        user,
        crate::host!(),
        community
    )
    .execute(&data.pool)
    .await?;

    Ok(HttpResponse::Ok())
}

/// Remove a moderator from a community
#[delete("/internal/communities/{community}/moderators/{user}")]
pub(crate) async fn remove_community_moderator(
    identity: Identity,
    data: web::Data<AppData>,
    web::Path((community, user)): web::Path<(String, String)>,
) -> Result<impl Responder, Error> {
    // exit early if requesting user is not authorised
    let requesting_user = match identity.identity() {
        Some(s) => s,
        None => {
            // must be logged in to subscribe to a community
            return Ok(HttpResponse::Unauthorized());
        }
    };

    // requesting user must be a moderator to remove a moderator
    if !is_moderator(requesting_user, crate::host!(), &community, &data.pool).await? {
        return Ok(HttpResponse::Unauthorized());
    }

    // target user must be a moderator
    if !is_moderator(&user, crate::host!(), &community, &data.pool).await? {
        return Ok(HttpResponse::NotFound());
    }

    // remove user from moderators
    sqlx::query!(
        r#"
            DELETE FROM moderators
            WHERE username = $1
            AND host = $2
            AND community = $3
        "#,
        user,
        crate::host!(),
        community
    )
    .execute(&data.pool)
    .await?;

    Ok(HttpResponse::Ok())
}

#[cfg(test)]
mod test {
    use {
        crate::{
            models::internal::{Community, User, UserId},
            test::{new_user_login, ADDR},
        },
        actix_web::{
            client::Client,
            http::{header::CONTENT_TYPE, StatusCode},
        },
    };

    #[actix_rt::test]
    async fn get_community_fail() {
        let res = Client::new()
            .get(&format!("{}/internal/communities/doesntexist", *ADDR))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[actix_rt::test]
    async fn create_community_success() {
        let (client, username, cookie) = new_user_login().await;
        let user_ids = vec![UserId {
            username: username,
            host: crate::host!(),
        }];

        // Create community successfully
        let mut res = client
            .post(&format!("{}/internal/communities", *ADDR))
            .cookie(cookie)
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "id": "community0",
                        "title": "My Community!",
                        "description": "My community description"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let community: Community = res.json().await.unwrap();
        assert_eq!(community.id.as_str(), "community0");
        assert_eq!(community.title.as_str(), "My Community!");
        assert_eq!(community.description.as_str(), "My community description");
        assert_eq!(community.moderators, user_ids);

        // Get community succesfully
        let mut res = client
            .get(&format!("{}/internal/communities/community0", *ADDR))
            .send()
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);
        let community: Community = res.json().await.unwrap();
        assert_eq!(community.id.as_str(), "community0");
        assert_eq!(community.title.as_str(), "My Community!");
        assert_eq!(community.description.as_str(), "My community description");
        assert_eq!(community.moderators, user_ids);
    }

    #[actix_rt::test]
    async fn delete_community_success() {
        let (client, _, cookie) = new_user_login().await;

        // Create community successfully
        let res = client
            .post(&format!("{}/internal/communities", *ADDR))
            .cookie(cookie.clone())
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "id": "community1",
                        "title": "My Delete Community!",
                        "description": "My community description"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // Get community succesfully
        let res = client
            .get(&format!("{}/internal/communities/community1", *ADDR))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // Delete community successfully
        let res = client
            .delete(&format!("{}/internal/communities/community1", *ADDR))
            .cookie(cookie.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // Fail to get community
        let res = client
            .get(&format!("{}/internal/communities/community1", *ADDR))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[actix_rt::test]
    async fn subscribe_success() {
        let (client, username, cookie) = new_user_login().await;

        // Create community to subscribe to
        let res = client
            .post(&format!("{}/internal/communities", *ADDR))
            .cookie(cookie.clone())
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "id": "community4",
                        "title": "My Community!",
                        "description": "My community description"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // Subscribe to it
        let res = client
            .post(&format!(
                "{}/internal/communities/community4/subscribe",
                *ADDR
            ))
            .cookie(cookie.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // subscribing twice should return OK
        let res = client
            .post(&format!(
                "{}/internal/communities/community4/subscribe",
                *ADDR
            ))
            .cookie(cookie.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // User entity should now reflect subscription
        let mut res = client
            .get(&format!("{}/internal/users/{}", *ADDR, username))
            .cookie(cookie.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let user: User = res.json().await.unwrap();
        assert_eq!(user.username, username);
        assert_eq!(user.subscribed, vec!["community4".to_string()]);
        assert_eq!(user.moderates, vec!["community4".to_string()]);

        // Unsubscribe to it
        let res = client
            .delete(&format!(
                "{}/internal/communities/community4/subscribe",
                *ADDR
            ))
            .cookie(cookie.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // User entity should now reflect subscription
        let mut res = client
            .get(&format!("{}/internal/users/{}", *ADDR, username))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let user: User = res.json().await.unwrap();
        assert_eq!(user.username, username);
        assert_eq!(user.subscribed.len(), 0);
        assert_eq!(user.moderates, vec!["community4".to_string()]);
    }

    #[actix_rt::test]
    async fn add_moderator_success() {
        let (moderator_client, moderator, moderator_cookie) = new_user_login().await;
        let (client, user, _) = new_user_login().await;

        // create the community
        let res = moderator_client
            .post(&format!("{}/internal/communities", *ADDR))
            .cookie(moderator_cookie.clone())
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "id": "add_mod_success",
                        "title": "Test Community",
                        "description": "🥰✨😘🙌"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // creator is a moderator by defualt
        let mut res = client
            .get(&format!("{}/internal/communities/add_mod_success", *ADDR))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let moderators = res.json::<Community>().await.unwrap().moderators;
        assert_eq!(moderators.len(), 1);
        assert_eq!(moderators[0].username, moderator);

        // add user as moderator
        let res = moderator_client
            .post(&format!(
                "{}/internal/communities/add_mod_success/moderators/{}",
                *ADDR, &user
            ))
            .cookie(moderator_cookie.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // now user is also a moderator
        let mut res = client
            .get(&format!("{}/internal/communities/add_mod_success", *ADDR))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let moderators = res.json::<Community>().await.unwrap().moderators;
        assert_eq!(moderators.len(), 2);
        assert!(moderators.contains(&UserId {
            username: moderator.clone(),
            host: crate::host!()
        }));
        assert!(moderators.contains(&UserId {
            username: user.clone(),
            host: crate::host!()
        }));
    }

    #[actix_rt::test]
    async fn add_already_moderator_success() {
        let (moderator_client, moderator, moderator_cookie) = new_user_login().await;
        let (moderator2_client, moderator2, moderator2_cookie) = new_user_login().await;

        // create the community
        let res = moderator_client
            .post(&format!("{}/internal/communities", *ADDR))
            .cookie(moderator_cookie.clone())
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "id": "add_alreadymod",
                        "title": "Test Community",
                        "description": "🥰✨😘🙌"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // add moderator2 as moderator
        let res = moderator_client
            .post(&format!(
                "{}/internal/communities/add_alreadymod/moderators/{}",
                *ADDR, &moderator2
            ))
            .cookie(moderator_cookie.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // now user is also a moderator
        let mut res = moderator_client
            .get(&format!("{}/internal/communities/add_alreadymod", *ADDR))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let moderators = res.json::<Community>().await.unwrap().moderators;
        assert_eq!(moderators.len(), 2);
        assert!(moderators.contains(&UserId {
            username: moderator.clone(),
            host: crate::host!()
        }));
        assert!(moderators.contains(&UserId {
            username: moderator2.clone(),
            host: crate::host!()
        }));

        // add moderator as moderator
        let res = moderator2_client
            .post(&format!(
                "{}/internal/communities/add_alreadymod/moderators/{}",
                *ADDR, &moderator
            ))
            .cookie(moderator2_cookie.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // check both are still moderators
        let mut res = moderator_client
            .get(&format!("{}/internal/communities/add_alreadymod", *ADDR))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let moderators = res.json::<Community>().await.unwrap().moderators;
        assert_eq!(moderators.len(), 2);
        assert!(moderators.contains(&UserId {
            username: moderator.clone(),
            host: crate::host!()
        }));
        assert!(moderators.contains(&UserId {
            username: moderator2.clone(),
            host: crate::host!()
        }));
    }

    #[actix_rt::test]
    async fn add_nonexistant_moderator_fail() {
        let (moderator_client, moderator, cookie) = new_user_login().await;

        // create the community
        let res = moderator_client
            .post(&format!("{}/internal/communities", *ADDR))
            .cookie(cookie.clone())
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "id": "add_noexist_mod_fail",
                        "title": "Test Community",
                        "description": "🥰✨😘🙌"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // creator is a moderator by defualt
        let mut res = moderator_client
            .get(&format!(
                "{}/internal/communities/add_noexist_mod_fail",
                *ADDR
            ))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let moderators = res.json::<Community>().await.unwrap().moderators;
        assert_eq!(moderators.len(), 1);
        assert_eq!(moderators[0].username, moderator);

        // add moderator as moderator
        let res = moderator_client
            .post(&format!(
                "{}/internal/communities/add_noexist_mod_fail/moderators/notarealuseridonotexist",
                *ADDR,
            ))
            .cookie(cookie.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[actix_rt::test]
    async fn add_moderator_notauthorized_fail() {
        let (moderator_client, moderator, cookie) = new_user_login().await;

        // create the community
        let res = moderator_client
            .post(&format!("{}/internal/communities", *ADDR))
            .cookie(cookie.clone())
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "id": "add_mod_noauth_fail",
                        "title": "Test Community",
                        "description": "🥰✨😘🙌"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // creator is a moderator by defualt
        let mut res = moderator_client
            .get(&format!(
                "{}/internal/communities/add_mod_noauth_fail",
                *ADDR
            ))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let moderators = res.json::<Community>().await.unwrap().moderators;
        assert_eq!(moderators.len(), 1);
        assert_eq!(moderators[0].username, moderator);

        // moderator removes themselves
        let res = moderator_client
            .delete(&format!(
                "{}/internal/communities/add_mod_noauth_fail/moderators/{}",
                *ADDR, &moderator
            ))
            .cookie(cookie.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // now there are no moderators
        let mut res = moderator_client
            .get(&format!(
                "{}/internal/communities/add_mod_noauth_fail",
                *ADDR
            ))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let moderators = res.json::<Community>().await.unwrap().moderators;
        assert_eq!(moderators.len(), 0);

        // non-moderator cannot make themselves moderator
        let res = moderator_client
            .post(&format!(
                "{}/internal/communities/add_mod_noauth_fail/moderators/{}",
                *ADDR, &moderator
            ))
            .cookie(cookie.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_rt::test]
    async fn remove_self_moderator_success() {
        let (moderator_client, moderator, cookie) = new_user_login().await;

        // create the community
        let res = moderator_client
            .post(&format!("{}/internal/communities", *ADDR))
            .cookie(cookie.clone())
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "id": "remove_self_mod_success",
                        "title": "Test Community",
                        "description": "🥰✨😘🙌"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // creator is a moderator by defualt
        let mut res = moderator_client
            .get(&format!(
                "{}/internal/communities/remove_self_mod_success",
                *ADDR
            ))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let moderators = res.json::<Community>().await.unwrap().moderators;
        assert_eq!(moderators.len(), 1);
        assert_eq!(moderators[0].username, moderator);

        // moderator removes themselves
        let res = moderator_client
            .delete(&format!(
                "{}/internal/communities/remove_self_mod_success/moderators/{}",
                *ADDR, &moderator
            ))
            .cookie(cookie.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[actix_rt::test]
    async fn remove_moderator_success() {
        let (moderator_client, moderator, moderator_cookie) = new_user_login().await;
        let (moderator2_client, moderator2, moderator2_cookie) = new_user_login().await;

        // create the community
        let res = moderator_client
            .post(&format!("{}/internal/communities", *ADDR))
            .cookie(moderator_cookie.clone())
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "id": "remove_moderator_success",
                        "title": "Test Community",
                        "description": "🥰✨😘🙌"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // add moderator2 as moderator
        let res = moderator_client
            .post(&format!(
                "{}/internal/communities/remove_moderator_success/moderators/{}",
                *ADDR, &moderator2
            ))
            .cookie(moderator_cookie.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // now user is also a moderator
        let mut res = moderator_client
            .get(&format!(
                "{}/internal/communities/remove_moderator_success",
                *ADDR
            ))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let moderators = res.json::<Community>().await.unwrap().moderators;
        assert_eq!(moderators.len(), 2);
        assert!(moderators.contains(&UserId {
            username: moderator.clone(),
            host: crate::host!()
        }));
        assert!(moderators.contains(&UserId {
            username: moderator2.clone(),
            host: crate::host!()
        }));

        // add moderator as moderator
        let res = moderator2_client
            .post(&format!(
                "{}/internal/communities/remove_moderator_success/moderators/{}",
                *ADDR, &moderator
            ))
            .cookie(moderator2_cookie.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // check both are still moderators
        let mut res = moderator_client
            .get(&format!(
                "{}/internal/communities/remove_moderator_success",
                *ADDR
            ))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let moderators = res.json::<Community>().await.unwrap().moderators;
        assert_eq!(moderators.len(), 2);
        assert!(moderators.contains(&UserId {
            username: moderator.clone(),
            host: crate::host!()
        }));
        assert!(moderators.contains(&UserId {
            username: moderator2.clone(),
            host: crate::host!()
        }));

        // moderator removes moderator2
        let res = moderator_client
            .delete(&format!(
                "{}/internal/communities/remove_moderator_success/moderators/{}",
                *ADDR, &moderator2
            ))
            .cookie(moderator_cookie.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // check moderator2 is no longer a moderator
        let mut res = moderator_client
            .get(&format!(
                "{}/internal/communities/remove_moderator_success",
                *ADDR
            ))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let moderators = res.json::<Community>().await.unwrap().moderators;
        assert_eq!(moderators.len(), 1);
        assert_eq!(moderators[0].username, moderator);
    }

    #[actix_rt::test]
    async fn remove_nonexistant_user_fail() {
        let (moderator_client, moderator, moderator_cookie) = new_user_login().await;
        let (client, _, _) = new_user_login().await;

        // create the community
        let res = moderator_client
            .post(&format!("{}/internal/communities", *ADDR))
            .cookie(moderator_cookie.clone())
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "id": "remove_non_user_fail",
                        "title": "Test Community",
                        "description": "🥰✨😘🙌"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // creator is a moderator by defualt
        let mut res = client
            .get(&format!(
                "{}/internal/communities/remove_non_user_fail",
                *ADDR
            ))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let moderators = res.json::<Community>().await.unwrap().moderators;
        assert_eq!(moderators.len(), 1);
        assert_eq!(moderators[0].username, moderator);

        // remove nonexistant user
        let res = moderator_client
            .delete(&format!(
                "{}/internal/communities/remove_non_user_fail/moderators/thisuserdoesnotexist",
                *ADDR
            ))
            .cookie(moderator_cookie.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[actix_rt::test]
    async fn remove_nonmoderator_fail() {
        let (moderator_client, moderator, moderator_cookie) = new_user_login().await;
        let (client, user, _) = new_user_login().await;

        // create the community
        let res = moderator_client
            .post(&format!("{}/internal/communities", *ADDR))
            .cookie(moderator_cookie.clone())
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "id": "remove_nonmoderator_fail",
                        "title": "Test Community",
                        "description": "🥰✨😘🙌"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // creator is a moderator by defualt
        let mut res = client
            .get(&format!(
                "{}/internal/communities/remove_nonmoderator_fail",
                *ADDR
            ))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let moderators = res.json::<Community>().await.unwrap().moderators;
        assert_eq!(moderators.len(), 1);
        assert_eq!(moderators[0].username, moderator);

        // remove non-moderator
        let res = moderator_client
            .delete(&format!(
                "{}/internal/communities/remove_nonmoderator_fail/moderators/{}",
                *ADDR, user
            ))
            .cookie(moderator_cookie.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }
}
