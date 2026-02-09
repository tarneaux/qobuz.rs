#![allow(clippy::unwrap_used)]
use qobuz::{
    auth::Credentials,
    downloader::{AutoRootDir, Download, DownloadConfig},
    Client,
};
use std::io::{self, Write};

#[tokio::main]
async fn main() {
    let client = Client::new(Credentials::from_env().unwrap()).await.unwrap();
    let conf = DownloadConfig::builder(AutoRootDir)
        .overwrite(true)
        .build()
        .unwrap();
    let let_it_be = client.get_track("129342731").await.unwrap();
    let (fut, progress_rx) = let_it_be.download(&conf, &client);
    tokio::spawn(async move {
        let rx = progress_rx.await;
        let Ok(mut rx) = rx else {
            return;
        };
        while rx.changed().await.is_ok() {
            let percent = {
                let progress = rx.borrow();
                (progress.downloaded * 100) / progress.total
            };
            print!("{percent}%\r");
            io::stdout().flush().unwrap();
        }
    });
    let path = fut.await.unwrap();
    println!("Downloaded {} to {}", let_it_be, path.display());
}
