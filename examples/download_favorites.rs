#![allow(clippy::unwrap_used)]

const DIR: &str = "music";

use qobuz::Downloader;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

use futures::stream;
use futures::StreamExt;
use qobuz::{Client, QobuzCredentials};
use qobuz::{Quality, Track};
use std::io::Write;

#[tokio::main]
async fn main() {
    let client = Client::new(QobuzCredentials::from_env().unwrap())
        .await
        .unwrap();
    let tracks: Vec<_> = client
        .get_user_favorites::<Track<()>>()
        .await
        .unwrap()
        .into_iter()
        .filter(|t| t.streamable)
        .collect();

    let downloader = Downloader::new(client.clone(), Path::new(DIR));

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
                let path = downloader
                    .download_and_tag_track(&t, &t.extra.album, Quality::Cd, false)
                    .await
                    .unwrap();
                *playlist.write().await.get_mut(i).unwrap() =
                    Some(path.to_str().unwrap().to_string());
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