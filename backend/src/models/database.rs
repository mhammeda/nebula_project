//! Database Models

use {
    serde::{Deserialize, Serialize},
    serde_json::Value,
    sqlx::FromRow,
    uuid::Uuid,
};

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct User {
    pub username: String,
    pub host: String,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct LocalUser {
    pub username: String,
    pub host: String,
    pub hash: String,
    pub recovery_key: String,
    pub created: i64,
    pub session: Option<Vec<u8>>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Community {
    pub id: String,
    pub title: String,
    pub description: String,
    pub created: i64,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Post {
    pub id: Uuid,
    pub community: String,
    pub parent: Option<Uuid>,
    pub author_username: String,
    pub author_host: String,
    pub title: String,
    pub content: Value, // JSON representation of Vec<PostContent>
    pub created: i64,
    pub modified: i64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub enum PostContent {
    Text(TextContent),
    Markdown(MarkdownContent),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TextContent {
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MarkdownContent {
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Message {
    pub id: Uuid,
    pub sender_username: String,
    pub sender_host: String,
    pub receiver_username: String,
    pub receiver_host: String,
    pub title: String,
    pub content: Value, // JSON representation of PostContent
    pub timestamp: i64,
    pub read: bool,
}
