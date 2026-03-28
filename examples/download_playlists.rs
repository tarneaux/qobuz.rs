#![allow(clippy::unwrap_used)]

use futures::{stream, StreamExt};
use qobuz::{
    auth::Credentials,
    downloader::{AutoRootDir, Download, DownloadConfig},
    Client,
};
use std::{
    io::{self, Write},
    path::{Path, PathBuf},
};

#[tokio::main]
async fn main() {
    let client = Client::new(Credentials::from_env().unwrap()).await.unwrap();
    let playlists = client.get_user_playlists().await.unwrap();

    let root_dir: PathBuf = AutoRootDir.into();

    let downloader = DownloadConfig::builder(Path::new(&root_dir))
        .build()
        .unwrap();

    let n = playlists.len();

    stream::iter(playlists)
        .enumerate()
        .for_each_concurrent(5, |(i, t)| {
            let client = client.clone();
            let downloader = downloader.clone();
            async move {
                let t = client
                    .get_playlist(t.id.to_string().as_str())
                    .await
                    .unwrap();
                println!("{}/{}: {}", i + 1, n, t);
                let (fut, progress_rx) = t.download(&downloader, &client);
                tokio::spawn(async move {
                    let rx = progress_rx.await;
                    let Ok(mut rx) = rx else {
                        return;
                    };
                    while rx.changed().await.is_ok() {
                        let percent = {
                            let progress = rx.borrow();
                            (progress.downloaded() * 100) / progress.total
                        };
                        print!("{percent}%\r");
                        io::stdout().flush().unwrap();
                    }
                });
                fut.await.unwrap();
            }
        })
        .await;
}
