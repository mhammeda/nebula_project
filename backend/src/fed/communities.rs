use {
    crate::{
        models::{
            database,
            fed::PostTimestamp,
            fed::{Community, UserId},
        },
        util::get_client_host,
        AppData, Error,
    },
    actix_web::{get, web, HttpRequest, HttpResponse, Responder, Result},
};

/// Gets a list of the IDs of communities on the server
#[get("/fed/communities")]
pub(crate) async fn get_communities(
    req: HttpRequest,
    data: web::Data<AppData>,
) -> Result<impl Responder, Error> {
    get_client_host(&req)?;

    // Fetch community IDs
    let rows: Vec<String> = sqlx::query!(
        r#"
            SELECT id
            FROM communities
        "#,
    )
    .fetch_all(&data.pool)
    .await?
    .into_iter()
    .map(|r| r.id)
    .collect();

    // Return a successful response containing the IDs in JSON
    Ok(HttpResponse::Ok().json(rows))
}

/// Gets a community by ID
#[get("/fed/communities/{id}")]
pub(crate) async fn get_community_by_id(
    req: HttpRequest,
    data: web::Data<AppData>,
    web::Path(id): web::Path<String>,
) -> Result<impl Responder, Error> {
    get_client_host(&req)?;

    // Fetch community
    let row = sqlx::query_as!(
        database::Community,
        r#"
            SELECT * FROM communities
            WHERE id = $1
        "#,
        id
    )
    .fetch_one(&data.pool)
    .await?;

    let mut community = Community {
        id: row.id.clone(),
        title: row.title,
        description: row.description,
        admins: vec![],
    };

    // fetch "admins"
    community.admins = sqlx::query!(
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
        id: r.username,
        host: r.host,
    })
    .collect();

    // Return a successful response containing the Community
    Ok(HttpResponse::Ok().json(community))
}

/// Gets the timestamps of last modification of all posts in a community
#[get("/fed/communities/{id}/timestamps")]
pub(crate) async fn get_community_timestamps(
    req: HttpRequest,
    data: web::Data<AppData>,
    web::Path(id): web::Path<String>,
) -> Result<impl Responder, Error> {
    get_client_host(&req)?;

    // check if community exists in order to return 404
    match sqlx::query!(
        r#"
            SELECT EXISTS(SELECT 1 FROM communities WHERE id = $1)
        "#,
        id
    )
    .fetch_one(&data.pool)
    .await?
    .exists
    {
        Some(true) => {}
        Some(false) => {
            return Ok(HttpResponse::NotFound().finish());
        }
        None => return Err(sqlx::error::Error::RowNotFound.into()),
    }

    // Execute query
    let rows: Vec<PostTimestamp> = sqlx::query!(
        r#"
            SELECT id, modified FROM posts
            WHERE community = $1
        "#,
        id
    )
    .fetch_all(&data.pool)
    .await?
    .into_iter()
    .map(|row| PostTimestamp {
        id: row.id,
        modified: row.modified,
    })
    .collect();

    Ok(HttpResponse::Ok().json(rows))
}
