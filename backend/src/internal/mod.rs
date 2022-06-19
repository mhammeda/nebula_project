//! Handlers for federated routes

mod admins;
mod communities;
mod images;
mod messages;
mod posts;
mod remotes;
mod users;
pub mod ws;

pub use {admins::*, communities::*, images::*, messages::*, posts::*, remotes::*, users::*};

#[cfg(test)]
mod test {
    use {
        crate::test::ADDR,
        actix_web::{client::Client, http::StatusCode},
    };

    #[actix_rt::test]
    async fn index_success() {
        let res = Client::new()
            .get(&format!("{}/", *ADDR))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[actix_rt::test]
    async fn invalid_path_index_success() {
        let res = Client::new()
            .get(&format!(
                "{}/thisisnotarealpathanddoesnotpointtoarealresource",
                *ADDR
            ))
            .send()
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);
    }
}
