use {
    crate::{
        models::internal::{CreatedUser, LoginInfo, NewUser, PasswordChange, User, UserId},
        util::user_exists,
        AppData, Error,
    },
    actix_identity::Identity,
    actix_web::{delete, get, post, put, web, HttpResponse, Responder, Result},
    rand::RngCore,
};

fn generate_password_hash<T: AsRef<[u8]>>(password: T) -> anyhow::Result<String, Error> {
    let mut rng = rand::thread_rng();
    let mut salt = [0u8; crate::SALT_LENGTH];
    rng.fill_bytes(&mut salt);

    let hash = argon2::hash_encoded(password.as_ref(), &salt, &argon2::Config::default())?;
    Ok(hash)
}

fn generate_recovery_key() -> anyhow::Result<(String, String), Error> {
    let mut rng = rand::thread_rng();
    let mut salt = [0u8; crate::SALT_LENGTH];
    rng.fill_bytes(&mut salt);

    let recovery_key = parity_wordlist::random_phrase(crate::RECOVERY_LENGTH);

    let recovery_key_hash =
        argon2::hash_encoded(recovery_key.as_bytes(), &salt, &argon2::Config::default())?;

    Ok((recovery_key, recovery_key_hash))
}

/// User login
#[post("/internal/login")]
pub(crate) async fn login(
    identity: Identity,
    data: web::Data<AppData>,
    web::Json(body): web::Json<LoginInfo>,
) -> Result<impl Responder, Error> {
    // Execute query
    let hash = sqlx::query!(
        r#"
            SELECT hash FROM local_users
            WHERE username = $1
            AND host = $2
        "#,
        body.username,
        crate::host!()
    )
    .fetch_one(&data.pool)
    .await?
    .hash;

    if argon2::verify_encoded(&hash, body.password.as_bytes())? {
        identity.remember(body.username);
        Ok(HttpResponse::Ok())
    } else {
        Ok(HttpResponse::Unauthorized())
    }
}

/// User logout
#[get("/internal/logout")]
pub(crate) async fn logout(identity: Identity) -> Result<impl Responder, Error> {
    match identity.identity() {
        Some(_) => {
            identity.forget();
            Ok(HttpResponse::Ok())
        }
        None => Ok(HttpResponse::Unauthorized()),
    }
}

/// Create new user
#[post("/internal/users")]
pub(crate) async fn create_user(
    identity: Identity,
    data: web::Data<AppData>,
    web::Json(body): web::Json<NewUser>,
) -> Result<impl Responder, Error> {
    if identity.identity().is_some() {
        // must be logged out to create a new user
        return Ok(HttpResponse::Unauthorized().finish());
    }

    if user_exists(&body.username, crate::host!(), &data.pool).await? {
        // username already in use
        return Ok(HttpResponse::BadRequest().finish());
    }

    match body.password.len() {
        8..=64 => {}
        _ => {
            // password requirements not met
            return Ok(HttpResponse::BadRequest().finish());
        }
    }

    let password_hash = generate_password_hash(body.password)?;
    let (recovery_key, recovery_key_hash) = generate_recovery_key()?;
    let now = chrono::Local::now().timestamp();

    sqlx::query!(
        r#"
            INSERT INTO users
            VALUES ($1, $2)
        "#,
        body.username,
        crate::host!()
    )
    .execute(&data.pool)
    .await?;

    sqlx::query!(
        r#"
            INSERT INTO local_users
            VALUES ($1, $2, $3, $4, $5, NULL)
        "#,
        body.username,
        crate::host!(),
        password_hash,
        recovery_key_hash,
        now
    )
    .execute(&data.pool)
    .await?;

    Ok(HttpResponse::Ok().json(CreatedUser {
        username: body.username.clone(),
        subscribed: vec![],
        moderates: vec![],
        created: now,
        recovery_key,
    }))
}

/// Get user information
#[get("/internal/users/{id}")]
pub(crate) async fn get_user(
    data: web::Data<AppData>,
    web::Path(username): web::Path<String>,
) -> Result<impl Responder, Error> {
    // Execute query
    let row = sqlx::query!(
        r#"
            SELECT created, avatar_url FROM local_users
            WHERE username = $1
            AND host = $2
        "#,
        username,
        crate::host!()
    )
    .fetch_one(&data.pool)
    .await?;

    let mut user = User {
        username: username.clone(),
        subscribed: vec![],
        moderates: vec![],
        created: row.created,
        avatar_url: row.avatar_url,
    };

    user.subscribed = sqlx::query!(
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
    .map(|row| row.community)
    .collect();

    user.moderates = sqlx::query!(
        r#"
            SELECT community FROM moderators
            WHERE username = $1
            AND host = $2
        "#,
        username,
        crate::host!()
    )
    .fetch_all(&data.pool)
    .await?
    .into_iter()
    .map(|row| row.community)
    .collect();

    // Return a successful response containing the User
    Ok(HttpResponse::Ok().json(user))
}

/// Delete a user
#[delete("/internal/users/{id}")]
pub(crate) async fn delete_user(
    identity: Identity,
    data: web::Data<AppData>,
    web::Path(username): web::Path<String>,
) -> Result<impl Responder, Error> {
    match identity.identity() {
        Some(s) => {
            if s != username {
                // must be logged in as user being deleted
                return Ok(HttpResponse::Unauthorized());
            }
        }
        None => {
            // must be logged in to delete account
            return Ok(HttpResponse::Unauthorized());
        }
    }

    sqlx::query!(
        r#"
            DELETE FROM users
            WHERE username = $1
            AND host = $2
        "#,
        username,
        crate::host!()
    )
    .execute(&data.pool)
    .await?;

    Ok(HttpResponse::Ok())
}

/// Change password
#[post("/internal/users/{id}/password")]
pub(crate) async fn change_user_password(
    identity: Identity,
    data: web::Data<AppData>,
    web::Path(username): web::Path<String>,
    web::Json(body): web::Json<PasswordChange>,
) -> Result<impl Responder, Error> {
    let mut new_recovery_key = None;
    let mut new_recovery_hash = None;

    match (identity.identity(), body.recovery_key) {
        (None, None) => {
            // Must be either logged in or provide a recovery key to change password
            return Ok(HttpResponse::Unauthorized());
        }
        // Recovery key based password change
        (None, Some(recovery_key)) => {
            // Execute query
            let hash = sqlx::query!(
                r#"
                    SELECT recovery_hash FROM local_users
                    WHERE username = $1
                    AND host = $2
                "#,
                username,
                crate::host!()
            )
            .fetch_one(&data.pool)
            .await?
            .recovery_hash;

            // Return error if the supplied recovery key is invalid
            if !argon2::verify_encoded(&hash, recovery_key.as_bytes())? {
                return Ok(HttpResponse::Unauthorized());
            }

            // Generate new recovery key
            let (recovery_key, recovery_hash) = generate_recovery_key()?;
            new_recovery_key = Some(recovery_key);
            new_recovery_hash = Some(recovery_hash);
        }
        // Use identity if available, ignore presence of recovery key
        (Some(identity), _) => {
            if identity != username {
                // logged in as wrong user
                return Ok(HttpResponse::Unauthorized());
            }
        }
    };

    let password_hash = generate_password_hash(body.password)?;

    // Update password
    sqlx::query!(
        r#"
            UPDATE local_users
            SET hash = $3, recovery_hash = COALESCE($4, recovery_hash)
            WHERE username = $1
            AND host = $2
        "#,
        username,
        crate::host!(),
        password_hash,
        new_recovery_hash
    )
    .execute(&data.pool)
    .await?;

    match new_recovery_key {
        Some(key) => Ok(HttpResponse::Ok().json(key).into()),
        None => Ok(HttpResponse::Ok()),
    }
}

#[put("/internal/users/{id}/avatar")]
pub(crate) async fn update_avatar_url(
    identity: Identity,
    data: web::Data<AppData>,
    web::Path(username): web::Path<String>,
    web::Json(url): web::Json<String>,
) -> Result<impl Responder, Error> {
    match identity.identity() {
        Some(s) => {
            if s != username {
                // must be logged in as user being updated
                return Ok(HttpResponse::Unauthorized());
            }
        }
        None => {
            // must be logged in to update url
            return Ok(HttpResponse::Unauthorized());
        }
    }

    // Update avatar url
    sqlx::query!(
        r#"
            UPDATE local_users
            SET avatar_url = $1
            WHERE username = $2
            AND host = $3
        "#,
        url,
        username,
        crate::host!(),
    )
    .execute(&data.pool)
    .await?;

    Ok(HttpResponse::Ok())
}

/// Fuzzy string search by username
#[get("/internal/users/search/{search}")]
pub(crate) async fn search_users(
    data: web::Data<AppData>,
    web::Path(search): web::Path<String>,
) -> Result<impl Responder, Error> {
    let users: Vec<UserId> = sqlx::query_as!(
        UserId,
        r#"
            SELECT * FROM users
            WHERE username % $1
        "#,
        search
    )
    .fetch_all(&data.pool)
    .await?;

    Ok(HttpResponse::Ok().json(users))
}

#[cfg(test)]
mod test {
    use {
        crate::{models::internal::CreatedUser, test::ADDR},
        actix_web::HttpMessage,
        awc::{
            http::{header::CONTENT_TYPE, StatusCode},
            Client,
        },
    };

    #[actix_rt::test]
    async fn create_user() {
        let client = Client::new();
        let res = client
            .post(&format!("{}/internal/users", *ADDR))
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "username": "user1",
                        "password": "passwordforuser1"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[actix_rt::test]
    async fn login_fail() {
        let client = Client::new();

        // Create a user
        let res = client
            .post(&format!("{}/internal/users", *ADDR))
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "username": "user2",
                        "password": "passwordforuser2"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // Incorrect password should return UNAUTHORIZED
        let res = client
            .post(&format!("{}/internal/login", *ADDR))
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "username": "user2",
                        "password": "incorrectpassowrd"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

        // Incorrect username should return NOTFOUND
        let res = client
            .post(&format!("{}/internal/login", *ADDR))
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "username": "nonexistinguser",
                        "password": "passwordforuser"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[actix_rt::test]
    async fn login_success() {
        let client = Client::new();

        // Create a user
        let res = client
            .post(&format!("{}/internal/users", *ADDR))
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "username": "user3",
                        "password": "passwordforuser3"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // Login should return OK with an auth cookie
        let res = client
            .post(&format!("{}/internal/login", *ADDR))
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "username": "user3",
                        "password": "passwordforuser3"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let cookies = res.cookies().unwrap();
        assert_eq!(cookies.len(), 1);
        assert_eq!(cookies[0].name(), "auth");
    }

    #[actix_rt::test]
    async fn logout_fail() {
        let client = Client::new();

        // Get request to logout should fail with BAD_REQUEST if cookies are missing
        let res = client
            .get(&format!("{}/internal/logout", *ADDR))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_rt::test]
    async fn logout_success() {
        let client = Client::new();

        // Create a user
        let res = client
            .post(&format!("{}/internal/users", *ADDR))
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "username": "user4",
                        "password": "passwordforuser4"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // Login succesfully
        let res = client
            .post(&format!("{}/internal/login", *ADDR))
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "username": "user4",
                        "password": "passwordforuser4"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let cookies = res.cookies().unwrap();
        assert_eq!(cookies.len(), 1);
        assert_eq!(cookies[0].name(), "auth");

        // Logout successfully
        let res = client
            .get(&format!("{}/internal/logout", *ADDR))
            .cookie(cookies[0].clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[actix_rt::test]
    async fn delete_user_fail() {
        let client = Client::new();

        // Create a user
        let res = client
            .post(&format!("{}/internal/users", *ADDR))
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "username": "user5",
                        "password": "passwordforuser5"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // Fail to delete user
        let res = client
            .delete(&format!("{}/internal/users/user5", *ADDR))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_rt::test]
    async fn delete_user_success() {
        let client = Client::new();

        // Create a user
        let res = client
            .post(&format!("{}/internal/users", *ADDR))
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "username": "user6",
                        "password": "passwordforuser6"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // Login succesfully
        let res = client
            .post(&format!("{}/internal/login", *ADDR))
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "username": "user6",
                        "password": "passwordforuser6"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let cookies = res.cookies().unwrap();
        assert_eq!(cookies.len(), 1);
        assert_eq!(cookies[0].name(), "auth");

        // Delete user successfully
        let res = client
            .delete(&format!("{}/internal/users/user6", *ADDR))
            .cookie(cookies[0].clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[actix_rt::test]
    async fn change_password_success() {
        let client = Client::new();

        // Create a user
        let res = client
            .post(&format!("{}/internal/users", *ADDR))
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "username": "user7",
                        "password": "passwordforuser7"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // Login should return fail if wrong password used
        let res = client
            .post(&format!("{}/internal/login", *ADDR))
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "username": "user7",
                        "password": "secondpasswordforuser7"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

        // Login should return OK with an auth cookie
        let res = client
            .post(&format!("{}/internal/login", *ADDR))
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "username": "user7",
                        "password": "passwordforuser7"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let cookies = res.cookies().unwrap();
        assert_eq!(cookies.len(), 1);
        assert_eq!(cookies[0].name(), "auth");

        // Change password
        let res = client
            .post(&format!("{}/internal/users/user7/password", *ADDR))
            .cookie(cookies[0].clone())
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "password": "secondpasswordforuser7"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // Logout
        let res = client
            .get(&format!("{}/internal/logout", *ADDR))
            .cookie(cookies[0].clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // Recreate client
        let client = Client::new();

        // Login should return fail if wrong password used
        let res = client
            .post(&format!("{}/internal/login", *ADDR))
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "username": "user7",
                        "password": "passwordforuser7"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

        // Login should return OK with an auth cookie with new password
        let res = client
            .post(&format!("{}/internal/login", *ADDR))
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "username": "user7",
                        "password": "secondpasswordforuser7"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let cookies = res.cookies().unwrap();
        assert_eq!(cookies.len(), 1);
        assert_eq!(cookies[0].name(), "auth");
    }

    #[actix_rt::test]
    async fn forgot_password_success() {
        let client = Client::new();

        // Create a user
        let mut res = client
            .post(&format!("{}/internal/users", *ADDR))
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "username": "user8",
                        "password": "passwordforuser8"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let recovery_key = res.json::<CreatedUser>().await.unwrap().recovery_key;

        // Login should return fail if wrong password used
        let res = client
            .post(&format!("{}/internal/login", *ADDR))
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "username": "user8",
                        "password": "secondpasswordforuser8"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

        // Login should return OK with an auth cookie
        let res = client
            .post(&format!("{}/internal/login", *ADDR))
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "username": "user8",
                        "password": "passwordforuser8"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let cookies = res.cookies().unwrap();
        assert_eq!(cookies.len(), 1);
        assert_eq!(cookies[0].name(), "auth");

        // Logout
        let res = client
            .get(&format!("{}/internal/logout", *ADDR))
            .cookie(cookies[0].clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // Recreate client
        let client = Client::new();

        // Change password with recovery key
        let res = client
            .post(&format!("{}/internal/users/user8/password", *ADDR))
            .header(CONTENT_TYPE, "application/json")
            .send_body(format!(
                "
                    {{
                        \"password\": \"secondpasswordforuser8\",
                        \"recoveryKey\":\"{}\"
                    }}
                ",
                recovery_key
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // Login should return fail if wrong password used
        let res = client
            .post(&format!("{}/internal/login", *ADDR))
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "username": "user8",
                        "password": "passwordforuser8"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

        // Login should return OK with an auth cookie with new password
        let res = client
            .post(&format!("{}/internal/login", *ADDR))
            .header(CONTENT_TYPE, "application/json")
            .send_body(
                r#"
                    {
                        "username": "user8",
                        "password": "secondpasswordforuser8"
                    }
                "#,
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let cookies = res.cookies().unwrap();
        assert_eq!(cookies.len(), 1);
        assert_eq!(cookies[0].name(), "auth");
    }
}
