use futures::StreamExt;
use qobuz::{Client, QobuzCredentials, Quality, Track};
use tokio::fs::File;
use tokio::{self};

#[tokio::main]
async fn main() {
    let client = Client::new(QobuzCredentials::from_env().unwrap())
        .await
        .unwrap();
    println!(
        "{}",
        client.get_user_favorites::<Track>().await.unwrap().items[0]
    );
    let mut bytes_stream = client
        .stream_track("64868955", Quality::HiRes96)
        .await
        .unwrap();
    let mut out = File::create("test.mp3")
        .await
        .expect("failed to create file");
    while let Some(item) = bytes_stream.next().await {
        tokio::io::copy(&mut item.unwrap().as_ref(), &mut out)
            .await
            .unwrap();
    }
}
