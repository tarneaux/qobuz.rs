use futures::StreamExt;
use qobuz::{Client, Quality};
use tokio::fs::File;
use tokio::{self};

use std::env;

#[tokio::main]
async fn main() {
    let email = env::var("EMAIL").expect("No $EMAIL");
    let password = env::var("PASSWORD").expect("No $PASSWORD");
    let app_id = env::var("APP_ID").expect("No $APP_ID");
    let secret = env::var("SECRET").expect("No $SECRET");
    println!("Got env vars, now logging in.");
    let client = Client::new(&email, &password, &app_id, secret)
        .await
        .unwrap();
    let url = client
        .get_track_file_url("64868955", Quality::HiRes96)
        .await
        .unwrap();
    let mut byte_stream = client
        .reqwest_client
        .get(url.inner)
        .send()
        .await
        .unwrap()
        .bytes_stream();
    let mut out = File::create("test.mp3")
        .await
        .expect("failed to create file");
    while let Some(item) = byte_stream.next().await {
        tokio::io::copy(&mut item.unwrap().as_ref(), &mut out)
            .await
            .unwrap();
    }
}
