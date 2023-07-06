mod core;
use crate::core::ApiClient;
use tokio;

use std::env;

#[tokio::main]
async fn main() {
    let email = env::var("EMAIL").unwrap();
    let password = env::var("PASSWORD").unwrap();
    let app_id = env::var("APP_ID").unwrap();
    let client = ApiClient::new(&email, &password, &app_id).await.unwrap();
    println!("{:?}", client);
}
