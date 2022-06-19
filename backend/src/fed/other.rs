use {
    crate::{AppData, Error},
    actix_web::{get, web, HttpResponse, Responder, Result},
    rsa::{PublicKeyPemEncoding, RSAPublicKey},
};

#[get("/fed/key")]
pub(crate) async fn get_public_key(data: web::Data<AppData>) -> Result<impl Responder, Error> {
    let pubkey = RSAPublicKey::from(&data.privkey);
    let pem = pubkey.to_pem_pkcs8().map_err(|e| Error::Parse(e.into()))?;

    Ok(HttpResponse::Ok()
        .content_type("application/x-pem-file")
        .body(pem))
}

#[get("/fed/discover")]
pub(crate) async fn get_known_hosts(data: web::Data<AppData>) -> Result<impl Responder, Error> {
    // pretty reasonable approximation for all foreign servers this server could know of
    let hosts: Vec<String> = sqlx::query!(
        r#"
            SELECT DISTINCT host FROM users
        "#,
    )
    .fetch_all(&data.pool)
    .await?
    .into_iter()
    .map(|r| r.host)
    .collect();

    Ok(HttpResponse::Ok().json(hosts))
}
