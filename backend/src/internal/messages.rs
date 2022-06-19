use {
    crate::{
        models::{
            database, fed,
            internal::{self, UserId},
        },
        util::user_exists,
        AppData, Error,
    },
    actix_identity::Identity,
    actix_web::{get, post, put, web, HttpResponse, Responder, Result},
    std::{
        borrow::Cow,
        collections::HashSet,
        convert::{TryFrom, TryInto},
    },
    uuid::Uuid,
};

/// Returns an array of IDs of the users that currently have unread chats with the requesting user
#[get("/internal/messages/unread")]
pub(crate) async fn get_unread(
    identity: Identity,
    data: web::Data<AppData>,
) -> Result<impl Responder, Error> {
    let username = match identity.identity() {
        Some(s) => s,
        None => return Ok(HttpResponse::Unauthorized().into()),
    };

    // select chats
    let users = sqlx::query!(
        r#"
            SELECT DISTINCT sender_username, sender_host FROM messages
            WHERE read = FALSE
            AND receiver_username = $1
            AND receiver_host = $2
        "#,
        username,
        crate::host!()
    )
    .fetch_all(&data.pool)
    .await?
    .into_iter()
    .map(|r| UserId {
        username: r.sender_username,
        host: r.sender_host,
    })
    .collect::<Vec<_>>();

    Ok(HttpResponse::Ok().json(users))
}

/// Returns an array of IDs of the users that currently have chats with the requesting user
#[get("/internal/messages")]
pub(crate) async fn get_all(
    identity: Identity,
    data: web::Data<AppData>,
) -> Result<impl Responder, Error> {
    let username = match identity.identity() {
        Some(s) => s,
        None => return Ok(HttpResponse::Unauthorized().into()),
    };

    // select chats
    let users = sqlx::query!(
        r#"
            SELECT DISTINCT sender_username, sender_host FROM messages
            WHERE receiver_username = $1
            AND receiver_host = $2
        "#,
        username,
        crate::host!()
    )
    .fetch_all(&data.pool)
    .await?
    .into_iter()
    .map(|r| UserId {
        username: r.sender_username,
        host: r.sender_host,
    })
    .chain(
        sqlx::query!(
            r#"
                SELECT DISTINCT receiver_username, receiver_host FROM messages
                WHERE sender_username = $1
                AND sender_host = $2
            "#,
            username,
            crate::host!()
        )
        .fetch_all(&data.pool)
        .await?
        .into_iter()
        .map(|r| UserId {
            username: r.receiver_username,
            host: r.receiver_host,
        }),
    )
    .collect::<HashSet<_>>();

    Ok(HttpResponse::Ok().json(users))
}

/// Mark a chat as read
#[put("/internal/messages/{user_id}/read")]
pub(crate) async fn mark_read(
    identity: Identity,
    data: web::Data<AppData>,
    web::Path(user_id): web::Path<String>,
) -> Result<impl Responder, Error> {
    let username = match identity.identity() {
        Some(s) => s,
        None => return Ok(HttpResponse::Unauthorized().into()),
    };

    let sender = UserId::try_from(user_id.as_str())?;

    sqlx::query!(
        r#"
            UPDATE messages
            SET read = true
            WHERE sender_username = $1
            AND sender_host = $2
            AND receiver_username = $3
            AND receiver_host = $4
        "#,
        sender.username,
        sender.host,
        username,
        crate::host!()
    )
    .execute(&data.pool)
    .await?;

    Ok(HttpResponse::Ok())
}

/// Get array of message objects belonging to the chat with the supplied user
#[get("/internal/messages/{user_id}")]
pub(crate) async fn get_messages_with_user(
    identity: Identity,
    data: web::Data<AppData>,
    web::Path(user_id): web::Path<String>,
) -> Result<impl Responder, Error> {
    let username = match identity.identity() {
        Some(s) => s,
        None => return Ok(HttpResponse::Unauthorized().into()),
    };

    // parse user_id from path
    let partner = UserId::try_from(user_id.as_str())?;

    let mut messages: Vec<internal::Message> = sqlx::query_as!(
        database::Message,
        r#"
            SELECT * FROM messages

            WHERE (sender_username = $1
            AND sender_host = $2
            AND receiver_username = $3
            AND receiver_host = $4)

            OR (sender_username = $3
            AND sender_host = $4
            AND receiver_username = $1
            AND receiver_host = $2)
        "#,
        partner.username,
        partner.host,
        username,
        crate::host!()
    )
    .fetch_all(&data.pool)
    .await?
    .into_iter()
    .map(|m| m.try_into())
    .collect::<Result<_, _>>()?;

    messages.sort_by(|a, b| a.timestamp.partial_cmp(&b.timestamp).unwrap());

    Ok(HttpResponse::Ok().json(messages))
}

/// Send a new message to a user
#[post("/internal/messages/{user_id}")]
pub(crate) async fn send_message_to_user(
    identity: Identity,
    data: web::Data<AppData>,
    web::Path(user_id): web::Path<String>,
    web::Json(body): web::Json<fed::Message>,
) -> Result<impl Responder, Error> {
    let id = match identity.identity() {
        Some(s) => s,
        None => return Ok(HttpResponse::Unauthorized().into()),
    };

    // parse user_id from path
    let receiver = UserId::try_from(user_id.as_str())?;

    if receiver.host == crate::host!() {
        // message is to local user, check that user exists
        if !user_exists(&receiver.username, &receiver.host, &data.pool).await? {
            return Ok(HttpResponse::NotFound().into());
        }
    } else {
        // message is to foreign user
        crate::Client::new(&data.privkey)
            .send_message(
                &id,
                &fed::UserId {
                    id: receiver.username.clone(),
                    host: receiver.host.clone(),
                },
                &body,
            )
            .await?;

        // ensure foreign user is in database
        match sqlx::query!(
            r#"
                INSERT INTO users VALUES ($1, $2)
            "#,
            receiver.username,
            receiver.host
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
    }

    let msg = internal::Message {
        id: Uuid::new_v4(),
        sender: UserId {
            username: id,
            host: crate::host!(),
        },
        receiver,
        title: body.title,
        content: body.content,
        timestamp: chrono::Local::now().timestamp(),
        read: false,
    };

    match sqlx::query!(
        r#"
            INSERT INTO messages VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
        msg.id,
        msg.sender.username,
        msg.sender.host,
        msg.receiver.username,
        msg.receiver.host,
        msg.title,
        serde_json::to_value(&msg.content)?,
        msg.timestamp,
        msg.read
    )
    .execute(&data.pool)
    .await
    {
        Ok(_) => Ok(HttpResponse::Created().json(msg)),
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod test {
    use {
        crate::{
            models::internal::UserId,
            test::{new_user_login, ADDR},
        },
        actix_http::http::{header::CONTENT_TYPE, StatusCode},
    };

    #[actix_rt::test]
    async fn send_message_success() {
        let (client, sender, cookie) = new_user_login().await;
        let (_, receiver, _) = new_user_login().await;

        let mut res = client
            .post(&format!("{}/internal/messages/{}", *ADDR, &receiver))
            .cookie(cookie.clone())
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "title": "🐝",
                        "content": {
                            "text": {
                                "text": "Hello friend!"
                            }
                        }
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        let res_body = res
            .json::<crate::models::internal::Message>()
            .await
            .unwrap();
        assert_eq!(
            res_body.sender,
            UserId {
                username: sender,
                host: crate::host!()
            }
        );
        assert_eq!(
            res_body.receiver,
            UserId {
                username: receiver,
                host: crate::host!()
            }
        );
        assert_eq!(res_body.title, "🐝".to_owned());
        assert_eq!(
            res_body.content,
            crate::models::database::PostContent::Text(crate::models::database::TextContent {
                text: "Hello friend!".to_owned()
            })
        );
    }

    #[actix_rt::test]
    async fn send_message_receiver_notexist_fail() {
        let (client, _, cookie) = new_user_login().await;

        let res = client
            .post(&format!("{}/internal/messages/thisuserdoesntexist", *ADDR))
            .cookie(cookie.clone())
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "title": "example title here",
                        "content": {
                            "text": {
                                "text": "🐝🐝🐝🐝🐝🐝"
                            }
                        }
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[actix_rt::test]
    async fn unread_mark_success() {
        let (sender_client, sender, sender_cookie) = new_user_login().await;
        let (receiver_client, receiver, receiver_cookie) = new_user_login().await;

        let res = sender_client
            .post(&format!("{}/internal/messages/{}", *ADDR, &receiver))
            .cookie(sender_cookie.clone())
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "title": "title goes here",
                        "content": {
                            "text": {
                                "text": "🦞🦑🦀"
                            }
                        }
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        let mut res = receiver_client
            .get(&format!("{}/internal/messages/unread", *ADDR))
            .cookie(receiver_cookie.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(
            res.json::<Vec<UserId>>().await.unwrap(),
            vec![UserId {
                username: sender.clone(),
                host: crate::host!()
            }]
        );

        let res = receiver_client
            .put(&format!("{}/internal/messages/{}/read", *ADDR, sender))
            .cookie(receiver_cookie.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let mut res = receiver_client
            .get(&format!("{}/internal/messages/unread", *ADDR))
            .cookie(receiver_cookie.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.json::<Vec<UserId>>().await.unwrap().len(), 0);
    }

    #[actix_rt::test]
    async fn get_chat_success() {
        let (sender_client, sender, sender_cookie) = new_user_login().await;
        let (receiver_client, receiver, receiver_cookie) = new_user_login().await;

        let res = sender_client
            .post(&format!("{}/internal/messages/{}", *ADDR, &receiver))
            .cookie(sender_cookie.clone())
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "title": "✨✨✨✨",
                        "content": {
                            "text": {
                                "text": "meeeesssssaaaaaagggggeeeeeeee"
                            }
                        }
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        let mut res = receiver_client
            .get(&format!("{}/internal/messages/{}", *ADDR, &sender))
            .cookie(receiver_cookie.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let res_body: Vec<crate::models::internal::Message> = res.json().await.unwrap();
        assert_eq!(res_body.len(), 1);
        assert_eq!(
            res_body[0].sender,
            UserId {
                username: sender,
                host: crate::host!()
            }
        );
        assert_eq!(
            res_body[0].receiver,
            UserId {
                username: receiver,
                host: crate::host!()
            }
        );
        assert_eq!(res_body[0].title, "✨✨✨✨".to_owned());
        assert_eq!(
            res_body[0].content,
            crate::models::database::PostContent::Text(crate::models::database::TextContent {
                text: "meeeesssssaaaaaagggggeeeeeeee".to_owned()
            })
        );
    }
}
