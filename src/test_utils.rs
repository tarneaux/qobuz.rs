#![allow(clippy::unwrap_used)]

use crate::{auth::Credentials, downloader::DownloadConfig, Client};
use std::path::Path;

pub async fn make_client() -> Client {
    let credentials = Credentials::from_env()
        .expect("Couldn't get credentials env variables which need to be set for this test.");
    Client::new(credentials)
        .await
        .expect("Couldn't create client with environment secrets")
}

pub fn make_download_config() -> DownloadConfig {
    let root_dir: &Path = Path::new("music");
    let m3u_dir = Path::new("music/playlists");

    if !m3u_dir.is_dir() {
        std::fs::create_dir_all(m3u_dir).unwrap();
    }

    DownloadConfig::builder(root_dir)
        .m3u_dir(m3u_dir.into())
        .overwrite(true)
        .build()
        .unwrap()
}
