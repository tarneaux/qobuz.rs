mod core;
use crate::core::ApiClient;
use futures::StreamExt;
use tokio::fs::File;
use tokio::{self};

use std::env;

#[tokio::main]
async fn main() {
    let email = env::var("EMAIL").expect("No $EMAIL");
    let password = env::var("PASSWORD").expect("No $PASSWORD");
    let app_id = env::var("APP_ID").expect("No $APP_ID");
    let secrets = env::var("SECRETS")
        .expect("No $SECRETS")
        .split(',')
        .map(|s| s.to_string())
        .collect();
    println!("Got env vars, now logging in.");
    let client = ApiClient::new(&email, &password, &app_id, secrets)
        .await
        .unwrap();
    println!("{:?}", client);
    let url = client
        .get_track_file_url("176991285")
        .await
        .unwrap()
        .unwrap();
    let mut byte_stream = client.client.get(url).send().await.unwrap().bytes_stream();
    let mut out = File::create("test.mp3")
        .await
        .expect("failed to create file");
    while let Some(item) = byte_stream.next().await {
        tokio::io::copy(&mut item.unwrap().as_ref(), &mut out)
            .await
            .unwrap();
    }
}
