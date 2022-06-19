//! Federation Security Middleware

use {
    crate::AppData,
    actix_http::error::PayloadError,
    actix_service::{Service, Transform},
    actix_web::{
        dev::{ServiceRequest, ServiceResponse},
        http::uri::Authority,
        web::{BytesMut, Data},
        Error, HttpMessage,
    },
    anyhow::{anyhow, Context as AnyhowContext, Result},
    async_stream::stream,
    futures_util::{
        future::{ok, Future, Ready},
        stream::StreamExt,
    },
    log::debug,
    rsa::{hash::Hash, padding::PaddingScheme, PublicKey, RSAPublicKey},
    sha2::{Digest, Sha512},
    std::{
        cell::RefCell,
        pin::Pin,
        rc::Rc,
        task::{Context, Poll},
    },
};

pub struct Signed;

impl<S: 'static, B> Transform<S> for Signed
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = SignedMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(SignedMiddleware {
            service: Rc::new(RefCell::new(service)),
        })
    }
}

pub struct SignedMiddleware<S> {
    // required to avoid lifetime issues
    service: Rc<RefCell<S>>,
}

impl<S, B> Service for SignedMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, mut req: ServiceRequest) -> Self::Future {
        let mut svc = self.service.clone();

        Box::pin(async move {
            // only validate signatures on non-key federation requests
            if req.uri().path().starts_with("/fed") && !req.uri().path().starts_with("/fed/key") {
                debug!(
                    "received federation request, validating signature: {:#?}",
                    req
                );
                validate_signature(&mut req)
                    .await
                    .map_err(|e| Error::from(crate::Error::BadRequest(e)))?;
                debug!("validated signature!");
            }

            Ok(svc.call(req).await?)
        })
    }
}

async fn validate_signature(req: &mut ServiceRequest) -> Result<()> {
    let input = gen_signature_input(req).await?;
    debug!("signature input: {}", &input);

    let pubkey = {
        let pool = req
            .app_data::<Data<AppData>>()
            .expect("Failed to get AppData")
            .pool
            .clone();

        let client_host = req
            .headers()
            .get("Client-Host")
            .ok_or(anyhow!("Missing Client-Host header"))?
            .to_str()?
            .parse::<Authority>()?
            .host()
            .to_owned();

        let der = sqlx::query!(
            r#"
                SELECT pubkey FROM remotes
                WHERE host = $1
            "#,
            client_host
        )
        .fetch_one(&pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => anyhow!(
                "Public key for host \"{}\" not found in database",
                client_host
            ),
            _ => e.into(),
        })?
        .pubkey;

        debug!("got pubkey for {}", client_host);

        RSAPublicKey::from_pkcs8(&der).expect("Keys in database should always be valid")
    };

    let signature = get_signature(req)?;

    debug!("got signature from request: {}", signature);

    pubkey
        .verify(
            PaddingScheme::PKCS1v15Sign {
                hash: Some(Hash::SHA2_512),
            },
            Sha512::digest(input.as_bytes()).as_slice(),
            &base64::decode(signature)?,
        )
        .context("Verifying signature")?;

    Ok(())
}

fn get_signature(req: &mut ServiceRequest) -> Result<String> {
    let value = req
        .headers()
        .get("Signature")
        .ok_or(anyhow!("Missing Signature header"))?
        .to_str()?
        .to_owned();

    Ok(value
        .split(',')
        .skip(3)
        .next()
        .ok_or(anyhow!(
            "Missing \"signature\" key in Signature header value"
        ))?
        .strip_prefix("signature=\"")
        .ok_or(anyhow!(
            "Missing \"signature\" value in Signature header value"
        ))?
        .strip_suffix('"')
        .ok_or(anyhow!("Failed to strip trailing doublequotes"))?
        .to_owned())
}

async fn gen_signature_input(req: &mut ServiceRequest) -> Result<String> {
    let mut digest = validate_digest(req).await?;
    // convert sha-512 to SHA-512
    digest.replace_range(..3, "SHA");

    let method = req.method().as_str().to_ascii_lowercase();
    let path = req.uri().path();
    let host = req
        .headers()
        .get("Host")
        .ok_or(anyhow!("Missing Host header"))?
        .to_str()?;
    let client_host = req
        .headers()
        .get("Client-Host")
        .ok_or(anyhow!("Missing Client-Host header"))?
        .to_str()?;
    let user_id = match req.headers().get("User-ID") {
        Some(value) => format!("user-id: {}\n", value.to_str()?),
        None => "".to_owned(),
    };
    let date = req
        .headers()
        .get("Date")
        .ok_or(anyhow!("Missing Date header"))?
        .to_str()?;

    Ok(format!(
        "(request-target): {} {}\nhost: {}\nclient-host: {}\n{}date: {}\ndigest: {}",
        method, path, host, client_host, user_id, date, digest
    ))
}

async fn validate_digest(req: &mut ServiceRequest) -> Result<String> {
    let mut body = BytesMut::new();
    let mut stream = req.take_payload();
    while let Some(chunk) = stream.next().await {
        body.extend_from_slice(&chunk?);
    }

    let digest = match req.headers().get("Digest") {
        Some(value) => {
            let digest = value.to_str()?.split("=").skip(1).next();

            let digest = match digest {
                Some(s) => s,
                _ => return Err(anyhow!("Digest header value badly formatted")),
            };

            let digest = match base64::decode(digest) {
                Ok(b) => b,
                Err(e) => return Err(anyhow!("Digest header value invalid base64: {:?}", e)),
            };

            if digest != Sha512::digest(&body).as_slice() {
                return Err(anyhow!(
                    "Invalid Digest header value: expected \"{:?}\" found \"{:?}\"",
                    &digest,
                    Sha512::digest(&body).as_slice()
                ));
            }

            base64::encode(Sha512::digest(&body))
        }
        None => return Err(anyhow!("Missing Digest header")),
    };
    debug!("digest validated: {:?}", digest);

    let stream = stream! {
        while !body.is_empty() {
            let out = body.split_to(if body.len() > 8192 { 8192 } else { body.len() });
            yield Ok::<_, PayloadError>(out.freeze());
        }
    };
    req.set_payload(actix_http::Payload::Stream(Box::pin(stream)));

    Ok(format!("sha-512={}", digest))
}
