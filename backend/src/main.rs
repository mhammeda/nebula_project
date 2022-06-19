#[actix_rt::main]
async fn main() {
    // load environment variables from .env file, failing silently
    dotenv::dotenv().ok();

    // build Config from environment variables and run backend
    backend::run(
        envy::from_env::<backend::Config>()
            .expect("Failed to get config from environment variables"),
    )
    .await
    .expect("Application error")
}
