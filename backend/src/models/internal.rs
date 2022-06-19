//! Internal Models

use {
    crate::{
        models::{
            database::{self, PostContent},
            fed,
        },
        Error,
    },
    regex::Regex,
    serde::{Deserialize, Serialize},
    std::convert::TryFrom,
    uuid::Uuid,
};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct UserId {
    pub username: String,
    pub host: String,
}

impl From<fed::UserId> for UserId {
    fn from(u: fed::UserId) -> Self {
        Self {
            username: u.id,
            host: u.host,
        }
    }
}

impl TryFrom<&str> for UserId {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let re = Regex::new("^([a-zA-Z0-9-_]{1,24})(@[a-zA-Z0-9-_.]{3,253}(:[0-9]{1,5})?)?$")
            .expect("Failed to build regular expression");

        let caps = match re.captures(value) {
            Some(c) => c,
            None => return Err(Self::Error::Parse(anyhow::anyhow!("UserId parse failed"))),
        };

        let username = match caps.get(1) {
            Some(s) => s.as_str().to_owned(),
            None => {
                return Err(Self::Error::Parse(anyhow::anyhow!(
                    "Username from UserId parse failed"
                )))
            }
        };

        let host = match caps.get(2) {
            Some(s) => s.as_str()[1..].to_owned(),
            None => crate::host!(),
        };

        Ok(UserId { username, host })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginInfo {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewUser {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub username: String,
    pub subscribed: Vec<String>,
    pub moderates: Vec<String>,
    pub created: i64,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedUser {
    pub username: String,
    pub subscribed: Vec<String>,
    pub moderates: Vec<String>,
    pub created: i64,
    pub recovery_key: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewCommunity {
    pub id: String,
    pub title: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Community {
    pub id: String,
    pub host: String,
    pub title: String,
    pub description: String,
    pub moderators: Vec<UserId>,
    pub created: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewPost {
    pub community: String,
    pub parent_post: Option<Uuid>,
    pub title: String,
    pub content: Vec<PostContent>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordChange {
    pub password: String,
    pub recovery_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Post {
    pub id: Uuid,
    pub host: String,
    pub community: String,
    pub parent_post: Option<Uuid>,
    pub children: Vec<Post>, // Nested children as opposed to UUIDs of children in fed::Post
    pub title: String,
    pub content: Vec<PostContent>,
    pub author: UserId,
    pub modified: i64,
    pub created: i64,
}

impl TryFrom<database::Post> for Post {
    type Error = Error;

    fn try_from(db: database::Post) -> Result<Self, self::Error> {
        Ok(Self {
            id: db.id,
            host: crate::host!(),
            community: db.community,
            parent_post: db.parent,
            children: vec![],
            title: db.title,
            content: serde_json::from_value(db.content)?,
            author: UserId {
                username: db.author_username,
                host: db.author_host,
            },
            created: db.created,
            modified: db.modified,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: Uuid,
    pub sender: UserId,
    pub receiver: UserId,
    pub title: String,
    pub content: PostContent,
    pub timestamp: i64,
    pub read: bool,
}

impl TryFrom<database::Message> for Message {
    type Error = Error;

    fn try_from(db: database::Message) -> Result<Self, self::Error> {
        Ok(Self {
            id: db.id,
            sender: UserId {
                username: db.sender_username,
                host: db.sender_host,
            },
            receiver: UserId {
                username: db.receiver_username,
                host: db.receiver_host,
            },
            title: db.title,
            content: serde_json::from_value(db.content)?,
            timestamp: db.timestamp,
            read: db.read,
        })
    }
}

#[cfg(test)]
mod test {
    use {super::UserId, proptest::prelude::*, std::convert::TryFrom};

    proptest! {
        #[test]
        fn userid_username_parse(username in "[a-zA-Z0-9-_]{1,24}") {
            // set in case these tests run before server initialisation
            crate::HOST.set("example.org".to_owned()).ok();

            let id = UserId::try_from(username.as_str()).unwrap();
            assert_eq!(id.username, username);
            assert_eq!(Some(&id.host), crate::HOST.get());
        }

        #[test]
        fn userid_hostname_parse(username in "[a-zA-Z0-9-_]{1,24}", domain in "[a-zA-Z0-9-_.]{3,253}") {
            // set in case these tests run before server initialisation
            crate::HOST.set("example.org".to_owned()).ok();

            let id = UserId::try_from(format!("{}@{}", username, domain).as_str()).unwrap();
            assert_eq!(id.username, username);
            assert_eq!(id.host, domain);
        }

        #[test]
        fn userid_hostname_with_port_parse(username in "[a-zA-Z0-9-_]{1,24}", domain in "[a-zA-Z0-9-_.]{3,253}", port in 0u16..65535) {
            // set in case these tests run before server initialisation
            crate::HOST.set("example.org".to_owned()).ok();

            let id = UserId::try_from(format!("{}@{}:{}", username, domain, port).as_str()).unwrap();
            assert_eq!(id.username, username);
            assert_eq!(id.host, format!("{}:{}", domain, port));
        }
    }
}
