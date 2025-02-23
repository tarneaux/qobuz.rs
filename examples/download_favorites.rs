#![allow(clippy::unwrap_used)]

const DIR: &str = "music";

use futures::{stream, StreamExt};
use qobuz::{
    auth::Credentials,
    downloader::{Download, DownloadConfig},
    types::{extra::WithExtra, Track},
    Client,
};
use std::{
    io::{self, Write},
    path::Path,
    sync::Arc,
};
use tokio::sync::RwLock;

#[tokio::main]
async fn main() {
    let client = Client::new(Credentials::from_env().unwrap()).await.unwrap();
    let tracks: Vec<_> = client
        .get_user_favorites::<Track<WithExtra>>()
        .await
        .unwrap()
        .into_iter()
        .filter(|t| t.streamable)
        .collect();

    let downloader = DownloadConfig::builder(Path::new(DIR)).build().unwrap();

    let n = tracks.len();
    let v = vec![None; n];
    let playlist: Arc<RwLock<Vec<Option<String>>>> = Arc::new(RwLock::new(v));

    stream::iter(tracks)
        .enumerate()
        .for_each_concurrent(1, |(i, t)| {
            let playlist = playlist.clone();
            let client = client.clone();
            let downloader = downloader.clone();
            async move {
                let t = client.get_track(t.id.to_string().as_str()).await.unwrap();
                println!("{}/{}: {}", i + 1, n, t.title);
                let (fut, res) = t.download(&downloader, &client);
                tokio::spawn(async move {
                    let mut rx = res.progress_rx.await.expect("No status returned");
                    while rx.changed().await.is_ok() {
                        let percent = {
                            let progress = rx.borrow();
                            (progress.downloaded * 100) / progress.total
                        };
                        print!("{percent}%\r");
                        io::stdout().flush().unwrap();
                    }
                });
                fut.await.unwrap();
                *playlist.write().await.get_mut(i).unwrap() =
                    Some(res.path.to_str().unwrap().to_string());
            }
        })
        .await;
    let playlist: Vec<String> = playlist
        .read()
        .await
        .iter()
        .map(|v| v.clone().unwrap())
        .collect();
    let mut f = std::fs::File::create(format!("{DIR}/favorites.m3u")).unwrap();
    write!(f, "{}", playlist.join("\n")).unwrap();
}
