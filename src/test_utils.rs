#![allow(clippy::unwrap_used)]

use crate::{
    auth::Credentials,
    downloader::{path_format::PathFormat, Downloader},
    quality::Quality,
    Client,
};
use std::path::Path;

pub async fn make_client() -> Client {
    let credentials = Credentials::from_env()
        .expect("Couldn't get credentials env variables which need to be set for this test.");
    Client::new(credentials)
        .await
        .expect("Couldn't create client with environment secrets")
}

pub async fn make_client_and_downloader() -> (Client, Downloader) {
    let music_path = Path::new("music");
    let playlist_path = Path::new("music/playlists");

    if !playlist_path.is_dir() {
        std::fs::create_dir_all(playlist_path).unwrap();
    }

    let client = make_client().await;
    (
        client.clone(),
        Downloader::new(
            client,
            music_path,
            playlist_path,
            Quality::Cd,
            true,
            PathFormat::default(),
        )
        .unwrap(),
    )
}
