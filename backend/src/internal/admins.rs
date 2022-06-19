use {
    crate::{models::internal::UserId, util::is_admin, AppData, Error},
    actix_identity::Identity,
    actix_web::{delete, get, post, web, HttpResponse, Responder, Result},
};

/// Gets list of all admins
#[get("/internal/admins")]
pub(crate) async fn get_admins(data: web::Data<AppData>) -> Result<impl Responder, Error> {
    // Execute query
    let admins = sqlx::query_as!(
        UserId,
        r#"
            SELECT * FROM admins
        "#
    )
    .fetch_all(&data.pool)
    .await?;

    Ok(HttpResponse::Ok().json(admins))
}

/// Gets admin status of a user
#[get("/internal/admins/{user_id}")]
pub(crate) async fn get_admin_status(
    data: web::Data<AppData>,
    web::Path(user_id): web::Path<String>,
) -> Result<impl Responder, Error> {
    if is_admin(&data.pool, user_id, crate::host!()).await? {
        return Ok(HttpResponse::Ok());
    } else {
        return Ok(HttpResponse::NotFound());
    }
}

/// Make a user an admin
#[post("/internal/admins/{user_id}")]
pub(crate) async fn add_admin(
    data: web::Data<AppData>,
    identity: Identity,
    web::Path(user_id): web::Path<String>,
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

    // no-op if target user is already an admin (covers the case that someone is adding themselves as an admin)
    if is_admin(&data.pool, &user_id, crate::host!()).await? {
        return Ok(HttpResponse::Ok());
    }

    // Execute query
    sqlx::query!(
        r#"
            INSERT INTO admins VALUES ($1, $2)
        "#,
        user_id,
        crate::host!()
    )
    .execute(&data.pool)
    .await?;

    Ok(HttpResponse::Ok())
}

/// Remove a user's admin rights
/// NOTE: it is possible for a user to lock themselves out by removing their own admin privileges
#[delete("/internal/admins/{user_id}")]
pub(crate) async fn remove_admin(
    data: web::Data<AppData>,
    identity: Identity,
    web::Path(user_id): web::Path<String>,
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

    // 404 if target user is not an admin
    if !is_admin(&data.pool, &user_id, crate::host!()).await? {
        return Ok(HttpResponse::NotFound());
    }

    // Execute query
    sqlx::query!(
        r#"
            DELETE FROM admins
            WHERE username = $1
            AND host = $2
        "#,
        user_id,
        crate::host!()
    )
    .execute(&data.pool)
    .await?;

    Ok(HttpResponse::Ok())
}

#[cfg(test)]
mod test {
    use {
        crate::test::{make_admin, new_user_login, ADDR},
        actix_web::{client::Client, http::StatusCode},
    };

    #[actix_rt::test]
    async fn get_admin_fail() {
        let res = Client::new()
            .get(&format!("{}/internal/admins/thisuserdoesnotexist", *ADDR))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[actix_rt::test]
    async fn get_nonadmin_success() {
        let (_, username, _) = new_user_login().await;

        let res = Client::new()
            .get(&format!("{}/internal/admins/{}", *ADDR, username))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[actix_rt::test]
    async fn get_admin_success() {
        let (_, username, _) = new_user_login().await;

        make_admin(&username, crate::host!()).await;

        let res = Client::new()
            .get(&format!("{}/internal/admins/{}", *ADDR, username))
            .send()
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);
    }

    #[actix_rt::test]
    async fn add_self_admin_fail() {
        let (client, username, _) = new_user_login().await;

        let res = client
            .post(&format!("{}/internal/admins/{}", *ADDR, &username))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_rt::test]
    async fn add_admin_success() {
        let (admin_client, admin, admin_cookie) = new_user_login().await;
        let (client, username, _) = new_user_login().await;

        make_admin(admin, crate::host!()).await;

        // not admin before
        let res = client
            .get(&format!("{}/internal/admins/{}", *ADDR, &username))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);

        // add admin
        let res = admin_client
            .post(&format!("{}/internal/admins/{}", *ADDR, &username))
            .cookie(admin_cookie)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // admin after
        let res = client
            .get(&format!("{}/internal/admins/{}", *ADDR, &username))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[actix_rt::test]
    async fn remove_admin_nonexistant_fail() {
        let (admin_client, admin, cookie) = new_user_login().await;

        make_admin(admin, crate::host!()).await;

        let res = admin_client
            .delete(&format!("{}/internal/admins/thisuserdoesnotexist", *ADDR))
            .cookie(cookie)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[actix_rt::test]
    async fn remove_admin_unauthorized_fail() {
        let (_, admin, _) = new_user_login().await;
        let (client, _, cookie) = new_user_login().await;

        make_admin(&admin, crate::host!()).await;

        // admin is admin
        let res = client
            .get(&format!("{}/internal/admins/{}", *ADDR, &admin))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // non admin cannot remove admin
        let res = client
            .delete(&format!("{}/internal/admins/{}", *ADDR, &admin))
            .cookie(cookie)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_rt::test]
    async fn remove_admin_notadmin_success() {
        let (admin_client, admin, admin_cookie) = new_user_login().await;
        let (client, user, _) = new_user_login().await;

        make_admin(&admin, crate::host!()).await;

        // admin is admin
        let res = client
            .get(&format!("{}/internal/admins/{}", *ADDR, &admin))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // user is not admin
        let res = client
            .get(&format!("{}/internal/admins/{}", *ADDR, &user))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);

        // cannot remove non-admin's admin privileges
        let res = admin_client
            .delete(&format!("{}/internal/admins/{}", *ADDR, &user))
            .cookie(admin_cookie)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[actix_rt::test]
    async fn remove_self_admin_success() {
        let (admin_client, admin, cookie) = new_user_login().await;

        make_admin(&admin, crate::host!()).await;

        // admin is admin
        let res = Client::new()
            .get(&format!("{}/internal/admins/{}", *ADDR, &admin))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // admin removes their own privileges
        let res = admin_client
            .delete(&format!("{}/internal/admins/{}", *ADDR, &admin))
            .cookie(cookie)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // admin is now not-admin
        let res = Client::new()
            .get(&format!("{}/internal/admins/{}", *ADDR, &admin))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[actix_rt::test]
    async fn remove_admin_success() {
        let (admin_client, admin, admin_cookie) = new_user_login().await;
        let (_, admin2, _) = new_user_login().await;

        make_admin(&admin, crate::host!()).await;
        make_admin(&admin2, crate::host!()).await;

        // admin is admin
        let res = Client::new()
            .get(&format!("{}/internal/admins/{}", *ADDR, &admin))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // admin2 is admin
        let res = Client::new()
            .get(&format!("{}/internal/admins/{}", *ADDR, &admin2))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // admin can delete admin2's admin privileges
        let res = admin_client
            .delete(&format!("{}/internal/admins/{}", *ADDR, &admin2))
            .cookie(admin_cookie)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // admin2 is now not admin
        let res = Client::new()
            .get(&format!("{}/internal/admins/{}", *ADDR, &admin2))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }
}
