use {
    crate::{AppData, Error},
    actix_identity::Identity,
    actix_multipart::Multipart,
    actix_web::{delete, get, post, web, HttpResponse, Responder, Result},
    anyhow::anyhow,
    futures_util::StreamExt,
    image::io::Reader as ImageReader,
    sqlx::Done,
    std::io::Cursor,
    uuid::Uuid,
};

const UPLOAD_SIZE_LIMIT: usize = 4 * 1024 * 1024;

/// Get image by id
#[get("/internal/images/{id}")]
pub(crate) async fn get_image(
    data: web::Data<AppData>,
    web::Path(id): web::Path<Uuid>,
) -> Result<impl Responder, Error> {
    let content = sqlx::query!(
        r#"
            SELECT content FROM images
            WHERE id = $1
        "#,
        id
    )
    .fetch_one(&data.pool)
    .await?
    .content;

    Ok(HttpResponse::Ok().content_type("image/jpeg").body(content))
}

/// Add an image
#[post("/internal/images")]
pub(crate) async fn add_image(
    data: web::Data<AppData>,
    identity: Identity,
    mut payload: Multipart,
) -> Result<impl Responder, Error> {
    // exit early if requesting user is not authorised
    let requesting_user = match identity.identity() {
        Some(s) => s,
        None => return Ok(HttpResponse::Unauthorized().into()),
    };

    let content = {
        // asynchronously read Multipart data
        let mut buf = vec![];
        if let Some(item) = payload.next().await {
            let mut field = item?;

            while let Some(chunk) = field.next().await {
                buf.extend_from_slice(&chunk?);
                if buf.len() > UPLOAD_SIZE_LIMIT {
                    return Err(Error::BadRequest(anyhow!("Exceeded file upload limit")));
                }
            }
        }

        // convert data to JPEG at 90% quality
        let mut out: Vec<u8> = vec![];
        let image = ImageReader::new(Cursor::new(buf))
            .with_guessed_format()
            .expect("Cursor I/O never fails")
            .decode()?;
        image.write_to(&mut out, image::ImageOutputFormat::Jpeg(90))?;

        out
    };

    let id = Uuid::new_v4();

    sqlx::query!(
        r#"
            INSERT INTO images VALUES ($1, $2, $3, $4)
        "#,
        id,
        content,
        requesting_user,
        crate::host!()
    )
    .execute(&data.pool)
    .await?;

    Ok(HttpResponse::Ok().json(id))
}

/// Remove an image
#[delete("/internal/images/{id}")]
pub(crate) async fn remove_image(
    data: web::Data<AppData>,
    identity: Identity,
    web::Path(id): web::Path<Uuid>,
) -> Result<impl Responder, Error> {
    // exit early if requesting user is not authorised
    let requesting_user = match identity.identity() {
        Some(s) => s,
        None => return Ok(HttpResponse::Unauthorized()),
    };

    let res = sqlx::query!(
        r#"
            DELETE FROM images
            WHERE images.id = $1
            AND images.author_username = $2
            AND images.author_host = $3
        "#,
        id,
        requesting_user,
        crate::host!()
    )
    .execute(&data.pool)
    .await?;

    if res.rows_affected() == 0 {
        return Ok(HttpResponse::NotFound());
    }

    Ok(HttpResponse::Ok())
}

#[cfg(test)]
mod test {
    use {
        crate::test::{new_user_login, ADDR},
        actix_http::HttpMessage,
        actix_multipart_rfc7578::client::multipart,
        actix_web::http::StatusCode,
        std::io::Cursor,
        uuid::Uuid,
    };

    #[actix_rt::test]
    async fn add_image_success() {
        let (client, _, cookie) = new_user_login().await;

        let mut form = multipart::Form::default();
        form.add_reader(
            "girl.png",
            Cursor::new(include_bytes!("testdata/in/girl.png")),
        );

        let mut res = client
            .post(&format!("{}/internal/images", *ADDR))
            .cookie(cookie.clone())
            .content_type(form.content_type())
            .send_body(multipart::Body::from(form))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let id = res.json::<Uuid>().await.unwrap();

        let mut res = client
            .get(&format!("{}/internal/images/{}", *ADDR, id))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.content_type(), "image/jpeg");
        assert_eq!(
            &res.body().await.unwrap()[..],
            &include_bytes!("testdata/out/girl.jpeg")[..]
        );
    }

    #[actix_rt::test]
    async fn add_image_invalid_fail() {
        let (client, _, cookie) = new_user_login().await;

        let mut form = multipart::Form::default();
        form.add_reader(
            "invalid",
            Cursor::new("Do I look like I know what a JPEG is?"),
        );

        let res = client
            .post(&format!("{}/internal/images", *ADDR))
            .cookie(cookie.clone())
            .content_type(form.content_type())
            .send_body(multipart::Body::from(form))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[actix_rt::test]
    async fn add_image_too_big_fail() {
        let (client, _, cookie) = new_user_login().await;

        let mut form = multipart::Form::default();
        form.add_reader("toobig", Cursor::new(vec![0u8; 5_000_000]));

        let res = client
            .post(&format!("{}/internal/images", *ADDR))
            .cookie(cookie.clone())
            .content_type(form.content_type())
            .send_body(multipart::Body::from(form))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[actix_rt::test]
    async fn remove_image_success() {
        let (client, _, cookie) = new_user_login().await;

        let mut form = multipart::Form::default();
        form.add_reader(
            "dog.jpg",
            Cursor::new(include_bytes!("testdata/in/dog.jpg")),
        );

        let mut res = client
            .post(&format!("{}/internal/images", *ADDR))
            .cookie(cookie.clone())
            .content_type(form.content_type())
            .send_body(multipart::Body::from(form))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let id = res.json::<Uuid>().await.unwrap();

        let mut res = client
            .get(&format!("{}/internal/images/{}", *ADDR, id))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.content_type(), "image/jpeg");
        assert_eq!(
            &res.body().await.unwrap()[..],
            &include_bytes!("testdata/out/dog.jpeg")[..]
        );

        let res = client
            .delete(&format!("{}/internal/images/{}", *ADDR, id))
            .cookie(cookie.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let res = client
            .get(&format!("{}/internal/images/{}", *ADDR, id))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[actix_rt::test]
    async fn remove_image_not_authorized_fail() {
        let (client, _, cookie) = new_user_login().await;

        let mut form = multipart::Form::default();
        form.add_reader("boat", Cursor::new(include_bytes!("testdata/in/boat.png")));

        let mut res = client
            .post(&format!("{}/internal/images", *ADDR))
            .cookie(cookie.clone())
            .content_type(form.content_type())
            .send_body(multipart::Body::from(form))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let id = res.json::<Uuid>().await.unwrap();

        let (client, _, cookie) = new_user_login().await;

        let res = client
            .delete(&format!("{}/internal/images/{}", *ADDR, id))
            .cookie(cookie.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[actix_rt::test]
    async fn remove_image_not_found_fail() {
        let (client, _, cookie) = new_user_login().await;

        let res = client
            .delete(&format!("{}/internal/images/{}", *ADDR, Uuid::new_v4()))
            .cookie(cookie.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }
}
