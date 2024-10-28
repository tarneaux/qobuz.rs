use futures::StreamExt;
use qobuz::{Client, QobuzCredentials, Quality, Track};
use tokio::fs::File;
use tokio::{self};

#[tokio::main]
async fn main() {
    let client = Client::new(QobuzCredentials::from_env().unwrap())
        .await
        .unwrap();
    let last_fav = client
        .get_user_favorites::<Track<()>>()
        .await
        .unwrap()
        .items[0]
        .clone();
    println!("{}", last_fav);
    println!("{:#?}", client.get_playlist("1848228").await);
    println!("{:#?}", client.get_album("0075596077460").await);
    let mut bytes_stream = client
        .stream_track(&last_fav.id.to_string(), Quality::HiRes96)
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
