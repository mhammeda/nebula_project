//! Federation Models

use {
    crate::{
        models::{database, internal},
        Error,
    },
    serde::{Deserialize, Serialize},
    std::convert::TryFrom,
    uuid::Uuid,
};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserId {
    pub id: String,
    pub host: String,
}

impl TryFrom<&str> for UserId {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let user_id = internal::UserId::try_from(value)?;
        Ok(Self {
            id: user_id.username,
            host: user_id.host,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Community {
    pub id: String,
    pub title: String,
    pub description: String,
    pub admins: Vec<UserId>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostTimestamp {
    pub id: Uuid,
    pub modified: i64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Post {
    pub id: Uuid,
    pub community: String,
    pub parent_post: Option<Uuid>,
    pub children: Vec<Uuid>,
    pub title: String,
    pub content: Vec<database::PostContent>,
    pub author: UserId,
    pub modified: i64,
    pub created: i64,
}

impl TryFrom<database::Post> for Post {
    type Error = Error;

    fn try_from(db: database::Post) -> Result<Self, self::Error> {
        Ok(Self {
            id: db.id,
            community: db.community,
            parent_post: db.parent,
            children: vec![],
            title: db.title,
            content: serde_json::from_value(db.content)?,
            author: UserId {
                id: db.author_username,
                host: db.author_host,
            },
            created: db.created,
            modified: db.modified,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewPost {
    pub community: String,
    pub parent_post: Option<Uuid>,
    pub title: Option<String>,
    pub content: Vec<database::PostContent>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostEdit {
    pub title: String,
    pub content: Vec<database::PostContent>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: String,
    pub posts: Vec<PostId>,
    pub about: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PostId {
    pub id: Uuid,
    pub host: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub title: String,
    pub content: database::PostContent,
}
