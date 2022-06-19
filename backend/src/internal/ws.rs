use {
    crate::{
        middleware::auth::validate_cookie,
        models::{database::PostContent, internal::UserId},
        AppData,
    },
    actix::prelude::*,
    actix_web::{get, web, HttpRequest, HttpResponse, Responder, Result},
    actix_web_actors::ws,
    log::{debug, error},
    serde::{Deserialize, Serialize},
    sqlx::{Pool, Postgres},
    std::{collections::HashMap, convert::TryFrom, time::Instant},
    uuid::Uuid,
};

/// Open a WebSocket connection
#[get("/ws/{auth}")]
pub(crate) async fn open_ws(
    data: web::Data<AppData>,
    req: HttpRequest,
    stream: web::Payload,
    web::Path(auth): web::Path<String>,
) -> Result<impl Responder, actix_web::Error> {
    let user_id = match validate_cookie(&auth, &data.secret, &data.pool).await? {
        Some(s) => UserId::try_from(s.as_str())?,
        None => {
            // must be logged in
            return Ok(HttpResponse::Unauthorized().into());
        }
    };

    ws::start(
        Session {
            user_id,
            heartbeat: Instant::now(),
            addr: data.ws_server.clone(),
        },
        &req,
        stream,
    )
}

#[derive(Debug)]
struct Session {
    user_id: UserId,
    heartbeat: Instant,
    addr: Addr<server::Server>,
}

impl Actor for Session {
    type Context = ws::WebsocketContext<Self>;

    /// Method called on actor start
    fn started(&mut self, ctx: &mut Self::Context) {
        // start heartbeat
        //self.hb(ctx);

        // connect to Server
        self.addr
            .send(server::Connect {
                user_id: self.user_id.clone(),
                addr: ctx.address().recipient(),
            })
            .into_actor(self)
            .then(|res, act, ctx| {
                match res {
                    Ok(_) => debug!("started WebSocket session: {:?}", act),
                    // something is wrong with chat server
                    Err(e) => {
                        error!("error while starting WebSocket session: {}", e);
                        ctx.stop()
                    }
                }
                fut::ready(())
            })
            .wait(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        debug!("stopping WebSocket session: {:?}", self);

        self.addr.do_send(server::Disconnect {
            user_id: self.user_id.clone(),
        });

        Running::Stop
    }
}

impl Handler<server::Message> for Session {
    type Result = ResponseFuture<Result<(), ()>>;

    fn handle(&mut self, msg: server::Message, ctx: &mut Self::Context) -> Self::Result {
        debug!("Session received message: {:?}", msg);

        if msg.receiver != self.user_id {
            error!(
                "user {:?} received messaged intended for {:?}",
                self.user_id, msg.receiver
            );
            return Box::pin(async { Ok(()) });
        }

        ctx.text(
            serde_json::to_string(&msg).expect("WebSocket message should never fail to serialize"),
        );

        Box::pin(async { Ok(()) })
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for Session {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        let msg = match msg {
            Err(_) => {
                ctx.stop();
                return;
            }
            Ok(msg) => msg,
        };

        debug!("received WebSocket message: {:?}", msg);
        match msg {
            ws::Message::Ping(msg) => {
                //self.hb = Instant::now();
                ctx.pong(&msg);
            }
            ws::Message::Pong(_) => {
                //self.hb = Instant::now();
            }
            ws::Message::Text(text) => {
                let m = text.trim();

                match serde_json::from_str::<server::Message>(m) {
                    Ok(msg) => {
                        if msg.sender != self.user_id {
                            error!(
                                "user {:?} tried to send a message as {:?}",
                                self.user_id, msg.sender
                            );
                        }

                        self.addr.do_send(msg.clone());
                    }
                    Err(e) => error!("failed to deserialise Message: {}", e),
                };
            }
            ws::Message::Binary(_) => error!("unexpected binary message in WebSocket"),
            ws::Message::Close(reason) => {
                ctx.close(reason);
                ctx.stop();
            }
            ws::Message::Continuation(_) => {
                ctx.stop();
            }
            ws::Message::Nop => (),
        }
    }
}

pub mod server {
    use super::*;

    #[derive(Debug)]
    pub struct Server {
        sessions: HashMap<String, Recipient<Message>>,
        pool: Pool<Postgres>,
    }

    impl Server {
        pub fn new(pool: Pool<Postgres>) -> Self {
            Self {
                sessions: HashMap::new(),
                pool,
            }
        }
    }

    impl Actor for Server {
        type Context = Context<Self>;
    }

    /// Handler for Connect message
    impl Handler<Connect> for Server {
        type Result = ();

        fn handle(&mut self, msg: Connect, _: &mut Context<Self>) {
            // insert address
            self.sessions.insert(msg.user_id.username, msg.addr);
        }
    }

    /// Handler for Disconnect message
    impl Handler<Disconnect> for Server {
        type Result = ();

        fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
            // remove address
            self.sessions.remove(&msg.user_id.username);
        }
    }

    /// Handler for Message message
    impl Handler<Message> for Server {
        type Result = ResponseFuture<Result<(), ()>>;

        fn handle(&mut self, msg: Message, _: &mut Context<Self>) -> Self::Result {
            if let Some(addr) = self.sessions.get(&msg.receiver.username) {
                let _ = addr.do_send(msg.clone());
            }

            // insert into db
            let pool = self.pool.clone();
            Box::pin(async move {
                sqlx::query!(
                    r#"
                        INSERT INTO messages VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                    "#,
                    Uuid::new_v4(),
                    msg.sender.username,
                    msg.sender.host,
                    msg.receiver.username,
                    msg.receiver.host,
                    "",
                    serde_json::to_value(&msg.content).unwrap(),
                    chrono::Local::now().timestamp(),
                    true
                )
                .execute(&pool)
                .await
                .unwrap();

                Ok(())
            })
        }
    }

    /// Message sent between Session and Server
    #[derive(Message, Debug, Clone, Serialize, Deserialize)]
    #[rtype(result = "Result<(), ()>")]
    pub struct Message {
        pub sender: UserId,
        pub receiver: UserId,
        pub content: PostContent,
    }

    #[derive(Message, Debug)]
    #[rtype(result = "()")]
    pub struct Connect {
        pub user_id: UserId,
        pub addr: Recipient<Message>,
    }

    #[derive(Message, Debug)]
    #[rtype(result = "()")]
    pub struct Disconnect {
        pub user_id: UserId,
    }
}
