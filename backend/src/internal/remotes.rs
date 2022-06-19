use {
    crate::{util::is_admin, AppData, Error},
    actix_identity::Identity,
    actix_web::{delete, get, http::uri::Authority, post, web, HttpResponse, Responder, Result},
};

/// Get current list of remote servers
#[get("/internal/remotes")]
pub(crate) async fn get_remote_servers(
    data: web::Data<AppData>,
    identity: Identity,
) -> Result<impl Responder, Error> {
    // exit early if requesting user is not authorised
    let requesting_user = match identity.identity() {
        Some(s) => s,
        None => return Ok(HttpResponse::Unauthorized().into()),
    };

    // check that requesting user is an admin
    if !is_admin(&data.pool, requesting_user, crate::host!()).await? {
        return Ok(HttpResponse::Unauthorized().into());
    }

    let remotes: Vec<String> = sqlx::query!(
        r#"
            SELECT host FROM remotes
        "#
    )
    .fetch_all(&data.pool)
    .await?
    .into_iter()
    .map(|r| r.host)
    .collect();

    Ok(HttpResponse::Ok().json(remotes))
}

/// Add a remote server
#[post("/internal/remotes/{remote}")]
pub(crate) async fn add_remote_server(
    data: web::Data<AppData>,
    identity: Identity,
    web::Path(remote): web::Path<String>,
) -> Result<impl Responder, Error> {
    // exit early if requesting user is not authorised
    let requesting_user = match identity.identity() {
        Some(s) => s,
        None => return Ok(HttpResponse::Unauthorized()),
    };

    // check that requesting user is an admin
    if !is_admin(&data.pool, requesting_user, crate::host!()).await? {
        return Ok(HttpResponse::Unauthorized());
    }

    // validate remote address and ignore port
    let authority = remote
        .parse::<Authority>()
        .map_err(|e| Error::BadRequest(e.into()))?;
    let host = authority.host();

    let pubkey = crate::Client::new(&data.privkey).get_key(&remote).await?;

    // Execute query
    sqlx::query!(
        r#"
            INSERT INTO remotes VALUES ($1, $2)
            ON CONFLICT (host) DO UPDATE
                SET pubkey = $2
        "#,
        host,
        pubkey
    )
    .execute(&data.pool)
    .await?;

    Ok(HttpResponse::Ok())
}

/// Remove a remote server
#[delete("/internal/remotes/{remote}")]
pub(crate) async fn remove_remote_server(
    data: web::Data<AppData>,
    identity: Identity,
    web::Path(remote): web::Path<String>,
) -> Result<impl Responder, Error> {
    // exit early if requesting user is not authorised
    let requesting_user = match identity.identity() {
        Some(s) => s,
        None => return Ok(HttpResponse::Unauthorized()),
    };

    // check that requesting user is an admin
    if !is_admin(&data.pool, requesting_user, crate::host!()).await? {
        return Ok(HttpResponse::Unauthorized());
    }

    let authority = remote
        .parse::<Authority>()
        .map_err(|e| Error::BadRequest(e.into()))?;
    let host = authority.host();

    // Execute query
    sqlx::query!(
        r#"
            DELETE FROM remotes
            WHERE host = $1
        "#,
        host
    )
    .execute(&data.pool)
    .await?;

    Ok(HttpResponse::Ok())
}

#[cfg(test)]
mod test {
    use {
        crate::test::{make_admin, new_user_login, ADDR},
        actix_http::http::StatusCode,
    };

    #[actix_rt::test]
    async fn add_remove_remote_success() {
        let (client, username, cookie) = new_user_login().await;

        // cannot see remotes if not an admin
        let res = client
            .get(&format!("{}/internal/remotes", *ADDR))
            .cookie(cookie.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

        make_admin(username, crate::host!()).await;

        // no remotes
        let mut res = client
            .get(&format!("{}/internal/remotes", *ADDR))
            .cookie(cookie.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(
            res.json::<Vec<String>>().await.unwrap(),
            Vec::<String>::new()
        );

        let host = ADDR.clone().strip_prefix("http://").unwrap().to_owned();

        // add own host (???)
        let res = client
            .post(&format!("{}/internal/remotes/{}", *ADDR, &host))
            .cookie(cookie.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // now there is one remote
        let mut res = client
            .get(&format!("{}/internal/remotes", *ADDR))
            .cookie(cookie.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(
            res.json::<Vec<String>>().await.unwrap(),
            vec![host.split(':').collect::<Vec<_>>()[0].clone()]
        );

        // update remote
        let res = client
            .post(&format!("{}/internal/remotes/{}", *ADDR, &host))
            .cookie(cookie.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // still only one remote
        let mut res = client
            .get(&format!("{}/internal/remotes", *ADDR))
            .cookie(cookie.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(
            res.json::<Vec<String>>().await.unwrap(),
            vec![host.split(':').collect::<Vec<_>>()[0].clone()]
        );

        // remove own host (???)
        let res = client
            .delete(&format!("{}/internal/remotes/{}", *ADDR, &host))
            .cookie(cookie.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // no remotes
        let mut res = client
            .get(&format!("{}/internal/remotes", *ADDR))
            .cookie(cookie.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(
            res.json::<Vec<String>>().await.unwrap(),
            Vec::<String>::new()
        );
    }

    #[actix_rt::test]
    async fn add_remote_bad_format_fail() {
        let (client, username, cookie) = new_user_login().await;

        make_admin(username, crate::host!()).await;

        // scheme not allowed
        let res = client
            .post(&format!(
                "{}/internal/remotes/http%3A%2F%2FFexample.org:1234",
                *ADDR
            ))
            .cookie(cookie.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        // path not allowed
        let res = client
            .post(&format!(
                "{}/internal/remotes/example.org:1234%2Fexample%2Fpath",
                *ADDR
            ))
            .cookie(cookie.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[actix_rt::test]
    async fn add_remote_bad_host_fail() {
        let (client, username, cookie) = new_user_login().await;

        make_admin(username, crate::host!()).await;

        // add example.org:1234
        let res = client
            .post(&format!("{}/internal/remotes/example.org:1234", *ADDR))
            .cookie(cookie.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[actix_rt::test]
    async fn add_remote_notadmin_fail() {
        let (client, _, _) = new_user_login().await;

        // add example.org:1234
        let res = client
            .post(&format!("{}/internal/remotes/example.org:1234", *ADDR))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
}
