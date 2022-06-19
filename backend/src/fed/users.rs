use {
    crate::{
        models::fed::{Message, PostId, User},
        util::{get_client_host, get_user_id, user_exists},
        AppData, Error,
    },
    actix_web::{get, post, web, HttpRequest, HttpResponse, Responder, Result},
    serde::Deserialize,
    std::borrow::Cow,
    uuid::Uuid,
};

/// Filters for GET /fed/users requests
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserFilters {
    prefix: Option<String>,
}

#[get("/fed/users")]
pub(crate) async fn get_users(
    data: web::Data<AppData>,
    web::Query(filters): web::Query<UserFilters>,
) -> Result<impl Responder, Error> {
    let users: Vec<String> = sqlx::query!(
        r#"
            SELECT username FROM users
            WHERE ($1::VARCHAR is null OR username ILIKE $1 || '%')
        "#,
        filters.prefix
    )
    .fetch_all(&data.pool)
    .await?
    .into_iter()
    .map(|r| r.username)
    .collect();

    Ok(HttpResponse::Ok().json(users))
}

#[get("/fed/users/{id}")]
pub(crate) async fn get_user_by_id(
    data: web::Data<AppData>,
    web::Path(id): web::Path<String>,
) -> Result<impl Responder, Error> {
    // check that user exists
    if !user_exists(&id, crate::host!(), &data.pool).await? {
        return Ok(HttpResponse::NotFound().into());
    }

    // fetch all post IDs authored by user
    let posts: Vec<PostId> = sqlx::query!(
        r#"
            SELECT id FROM posts
            WHERE author_username = $1
            AND author_host = $2
        "#,
        &id,
        crate::host!()
    )
    .fetch_all(&data.pool)
    .await?
    .into_iter()
    .map(|x| PostId {
        id: x.id,
        host: crate::host!(),
    })
    .collect();

    Ok(HttpResponse::Ok().json(User {
        id,
        posts,
        about: "".to_owned(),
        avatar_url: None,
    }))
}

#[post("/fed/users/{id}")]
pub(crate) async fn send_message(
    req: HttpRequest,
    data: web::Data<AppData>,
    web::Path(id): web::Path<String>,
    web::Json(body): web::Json<Message>,
) -> Result<impl Responder, Error> {
    let sender_id = get_user_id(&req)?;
    let sender_host = get_client_host(&req)?;

    (&id, crate::host!());

    // check that receiving user exists
    if !user_exists(&id, crate::host!(), &data.pool).await? {
        return Ok(HttpResponse::NotFound().into());
    }

    // ensure that sending user exists
    match sqlx::query!(
        r#"
            INSERT INTO users VALUES ($1, $2)
        "#,
        sender_id,
        sender_host
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

    // insert message
    match sqlx::query!(
        r#"
                INSERT INTO messages VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        Uuid::new_v4(),
        sender_id,
        sender_host,
        id,
        crate::host!(),
        body.title,
        serde_json::to_value(&body.content)?,
        chrono::Local::now().timestamp(),
        false
    )
    .execute(&data.pool)
    .await
    {
        Ok(_) => Ok(HttpResponse::Created()),
        Err(sqlx::Error::Database(e)) => {
            if e.code() == Some(Cow::from("23503")) {
                Ok(HttpResponse::NotFound().into())
            } else {
                Err(sqlx::Error::Database(e).into())
            }
        }
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod test {}
