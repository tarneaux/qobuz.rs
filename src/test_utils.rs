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
    let client = make_client().await;
    (
        client.clone(),
        Downloader::new(
            client,
            Path::new("music"),
            Path::new("music/playlists"),
            Quality::Cd,
            true,
            PathFormat::default(),
        )
        .unwrap(),
    )
}
