use {
    crate::{
        fed::PostFilters,
        models::fed::{Community, Message, Post, UserId},
    },
    actix_web::{
        client::{Client as ActixClient, ClientRequest, JsonPayloadError, SendRequestError},
        error::PayloadError,
        http::{
            header::{Date, ToStrError, LOCATION},
            uri::{Authority, InvalidUri, Parts, Scheme, Uri},
            StatusCode,
        },
        HttpMessage,
    },
    anyhow::anyhow,
    log::debug,
    rsa::{hash::Hash, PaddingScheme, RSAPrivateKey, RSAPublicKey},
    serde::{de::DeserializeOwned, Serialize},
    sha2::{Digest, Sha512},
    std::{
        collections::HashMap,
        convert::{TryFrom, TryInto},
        time::SystemTime,
    },
};

/// Federation Client
pub struct Client {
    client: ActixClient,
    privkey: RSAPrivateKey,
}

impl Client {
    /// Creates a new Client with the supplied private key
    pub fn new(privkey: &RSAPrivateKey) -> Self {
        Self {
            client: ActixClient::default(),
            privkey: privkey.clone(),
        }
    }

    async fn validate_host<T: AsRef<str>>(&self, host: T) -> Result<Parts, Error> {
        let host = Authority::try_from(host.as_ref())?;

        let mut parts = Parts::default();
        parts.scheme = Some(Scheme::HTTP);
        parts.authority = Some(host.clone());
        parts.path_and_query = Some(
            "/".try_into()
                .expect("Parsing index path should always succeed"),
        );

        let response = self.client.get(parts).send().await?;

        let mut parts = Parts::default();
        parts.scheme = Some(Scheme::HTTP);
        parts.authority = Some(host);
        parts.path_and_query = Some(
            "/".try_into()
                .expect("Parsing index path should always succeed"),
        );

        if response.status() == StatusCode::MOVED_PERMANENTLY {
            let location = response
                .headers()
                .get(LOCATION)
                .ok_or(anyhow!("Missing Location header"))
                .expect("Got 301 but no location")
                .to_str()
                .expect("Location should be valid UTF-8");
            parts = Parts::from(Uri::try_from(location)?);
        }

        Ok(parts)
    }

    async fn send_json<A: Serialize, B: DeserializeOwned>(
        &self,
        req: ClientRequest,
        value: &A,
    ) -> Result<B, Error> {
        let body = serde_json::to_string(value)?;

        debug!("body: {}", body);

        let digest = base64::encode(Sha512::digest(body.as_bytes()));

        debug!("generated digest: {}", digest);

        let user_id = match req.headers().get("User-ID") {
            Some(value) => format!("user-id: {}\n", value.to_str()?),
            None => "".to_owned(),
        };

        let req = req.set(Date(SystemTime::now().into()));
        let date = req
            .headers()
            .get("Date")
            .expect("Date header should not be missing")
            .to_str()?;

        let host = req
            .get_uri()
            .authority()
            .expect("URI should contain an authority");

        let signature_input = format!(
            "(request-target): {} {}\nhost: {}\nclient-host: {}\n{}date: {}\ndigest: SHA-512={}",
            req.get_method().as_str().to_lowercase(),
            req.get_uri().path(),
            host,
            crate::host!(),
            user_id,
            date,
            digest
        );

        let signature = format!(
            "keyId=\"global\",algorithm=\"rsa-sha512\",headers=\"(request-target) host client-host {}date digest\",signature={}",
            if user_id == "" {
                ""
            } else {
                "user-id "
            },
            base64::encode(self.privkey.sign(
                PaddingScheme::PKCS1v15Sign {
                    hash: Some(Hash::SHA2_512)
                },
                Sha512::digest(signature_input.as_bytes()).as_slice(),
            )?)
        );

        debug!("generated signature: {}", signature);

        let mut response = req
            .header("Client-Host", crate::host!())
            .header("Digest", format!("sha-512={}", digest))
            .header("Signature", signature.clone())
            .content_type("application/json")
            .send_body(&body)
            .await?;

        if !response.status().is_success() {
            match response.body().await {
                Ok(b) => match core::str::from_utf8(&b) {
                    Ok(s) => debug!("federation client request failed: {}", s),
                    _ => {}
                },
                _ => {}
            }

            return Err(Error::ResponseStatus(response.status()));
        }

        Ok(response.json().await?)
    }

    /// Gets the public key of a remote host
    pub async fn get_key<H: AsRef<str>>(&self, host: H) -> Result<Vec<u8>, Error> {
        let mut parts = self.validate_host(host.as_ref()).await?;
        parts.path_and_query = Some("/fed/key".try_into()?);

        let mut response = self
            .client
            .get(parts)
            .header("Client-Host", crate::host!())
            .send()
            .await?;

        if !response.status().is_success() {
            match response.body().await {
                Ok(b) => match core::str::from_utf8(&b) {
                    Ok(s) => debug!("federation client request failed: {}", s),
                    _ => {}
                },
                _ => {}
            }

            return Err(Error::ResponseStatus(response.status()));
        }

        if response.content_type() != "application/x-pem-file" {
            return Err(Error::ContentType(response.content_type().to_owned()));
        }

        // Load body
        let body_bytes = response.body().await?;

        // Parse body as &str
        let body = core::str::from_utf8(&body_bytes).map_err(|e| Error::Body(e.into()))?;

        // Remove PKCS8 headers
        let mut der_encoded = body.lines().filter(|line| !line.starts_with("-")).fold(
            String::new(),
            |mut data, line| {
                data.push_str(&line);
                data
            },
        );
        der_encoded.retain(|c| !c.is_whitespace());

        // Base64 decode DER
        let der = base64::decode(der_encoded).map_err(|e| Error::Body(e.into()))?;

        // Ensure that the key can be parsed
        debug!(
            "Successfully retrieved public key for {:?}: {:?}",
            host.as_ref(),
            RSAPublicKey::from_pkcs8(&der).map_err(|e| Error::Body(e.into()))?
        );

        Ok(der)
    }

    /// Sends a message to a host
    pub async fn send_message<T: AsRef<str>>(
        &self,
        from: T,
        to: &UserId,
        msg: &Message,
    ) -> Result<(), Error> {
        let mut parts = self.validate_host(&to.host).await?;

        parts.path_and_query = Some(format!("/fed/users/{}", to.id).try_into()?);

        self.send_json(
            self.client.post(parts).header("User-ID", from.as_ref()),
            msg,
        )
        .await?;

        debug!("fed client: sent message to {:?}", to);

        Ok(())
    }

    /// Gets a list of the IDs of communities on the server
    pub async fn get_communities<T: AsRef<str>>(&self, host: T) -> Result<Vec<String>, Error> {
        let mut parts = self.validate_host(host).await?;

        parts.path_and_query = Some("/fed/communities".try_into()?);

        let ids = self
            .send_json(self.client.get(parts), &HashMap::<(), ()>::with_capacity(0))
            .await?;

        debug!("fed client: got community ids: {:?}", ids);

        Ok(ids)
    }

    /// Gets a community by ID
    pub async fn get_community<H: AsRef<str>, C: AsRef<str>>(
        &self,
        host: H,
        community: C,
    ) -> Result<Community, Error> {
        let mut parts = self.validate_host(host).await?;

        parts.path_and_query = Some(format!("/fed/communities/{}", community.as_ref()).try_into()?);

        let community = self
            // serialising an empty hashmap to get send "{}" as the body of the request to avoid errors from body-parser in Express backends
            .send_json(self.client.get(parts), &HashMap::<(), ()>::with_capacity(0))
            .await?;

        debug!("fed client: got community: {:?}", community);

        Ok(community)
    }

    /// Gets all posts
    pub async fn get_posts<A: AsRef<str>, B: AsRef<str>>(
        &self,
        host: A,
        filters: PostFilters,
        user: B,
    ) -> Result<Vec<Post>, Error> {
        let mut parts = self.validate_host(host).await?;

        let mut query =
            serde_urlencoded::ser::to_string(filters).map_err(|e| Error::Body(e.into()))?;
        if !query.is_empty() {
            query = "?".to_owned() + &query;
        }

        parts.path_and_query = Some(format!("/fed/posts{}", query).try_into()?);

        let posts = self
            // serialising an empty hashmap to get send "{}" as the body of the request to avoid errors from body-parser in Express backends
            .send_json(
                self.client.get(parts).header("User-ID", user.as_ref()),
                &HashMap::<(), ()>::with_capacity(0),
            )
            .await?;

        debug!("fed client: got posts: {:?}", posts);

        Ok(posts)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Error during sending and response reading: {0:?}")]
    Send(SendRequestError),

    #[error("Received non-success status code: {0:?}")]
    ResponseStatus(StatusCode),

    #[error("Received unexpected content type: {0:?}")]
    ContentType(String),

    #[error("Invalid body")]
    Body(anyhow::Error),

    #[error("Remote host not found in database")]
    InvalidRemote,

    #[error("Database error: {0:?}")]
    Database(sqlx::Error),

    #[error("Payload error: {0:?}")]
    Payload(anyhow::Error),

    #[error("Error during request construction: {0:?}")]
    Construction(anyhow::Error),
}

impl From<SendRequestError> for Error {
    fn from(e: SendRequestError) -> Self {
        Self::Send(e)
    }
}

impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e)
    }
}

impl From<PayloadError> for Error {
    fn from(e: PayloadError) -> Self {
        Self::Payload(e.into())
    }
}

impl From<JsonPayloadError> for Error {
    fn from(e: JsonPayloadError) -> Self {
        Self::Payload(e.into())
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::Construction(e.into())
    }
}

impl From<ToStrError> for Error {
    fn from(e: ToStrError) -> Self {
        Self::Construction(e.into())
    }
}

impl From<rsa::errors::Error> for Error {
    fn from(e: rsa::errors::Error) -> Self {
        Self::Construction(e.into())
    }
}

impl From<InvalidUri> for Error {
    fn from(e: InvalidUri) -> Self {
        Self::Construction(e.into())
    }
}
