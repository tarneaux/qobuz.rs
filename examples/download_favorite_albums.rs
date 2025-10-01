#![allow(clippy::unwrap_used)]

use qobuz::auth::Credentials;
use qobuz::downloader::Download;
use qobuz::downloader::DownloadConfig;
use qobuz::types::extra::WithoutExtra;
use qobuz::types::Album;
use qobuz::Client;
use std::path::Path;

const PATH: &str = "music";

#[tokio::main]
async fn main() {
    let client = Client::new(Credentials::from_env().unwrap()).await.unwrap();
    let albums: Vec<_> = client
        .get_user_favorites::<Album<WithoutExtra>>()
        .await
        .unwrap()
        .into_iter()
        .filter(|t| t.streamable)
        .collect();
    for album in albums {
        println!("Downloading {album}");
        let downloader = DownloadConfig::builder(Path::new(PATH))
            .overwrite(true)
            .build()
            .unwrap();
        let album = client.get_album(&album.id).await.unwrap();
        let (fut, progress_rx) = album.download(&downloader, &client);
        tokio::spawn(async move {
            let mut rx = progress_rx.await.unwrap();
            while rx.changed().await.is_ok() {
                let progress = rx.borrow();
                println!("{progress:?}");
            }
        });
        fut.await.unwrap();
    }
}
