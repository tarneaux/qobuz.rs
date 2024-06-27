use futures::StreamExt;
use qobuz::{Client, QobuzCredentials, Quality, Track};
use tokio::fs::File;
use tokio::{self};


#[tokio::main]
async fn main() {
    println!("Got env vars, now logging in.");
    let client = Client::new(QobuzCredentials::from_env().unwrap())
        .await
        .unwrap();
    println!(
        "{}",
        client
            .get_user_favorites::<Track>()
            .await
            .unwrap()
            .items
            .len()
    );
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
