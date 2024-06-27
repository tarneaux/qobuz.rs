use futures::StreamExt;
use qobuz::{Album, Client, QobuzCredentials, Quality, Track};
use tokio::fs::File;
use tokio::{self};

use std::env;

#[tokio::main]
async fn main() {
    println!("Got env vars, now logging in.");
    let client = Client::new(QobuzCredentials::from_env().unwrap())
        .await
        .unwrap();
    println!("{:?}", client.get_playlist("22489221").await.unwrap());
    println!("{:?}", client.get_track("176991285").await.unwrap());
    println!("{:?}", client.get_user_favorites::<Album>().await.unwrap());
    println!("{:?}", client.get_album("jojel62htsvkc").await.unwrap());
    println!("{:?}", client.get_artist("35074").await.unwrap());
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
