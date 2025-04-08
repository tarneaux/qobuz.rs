use clap::{Parser, Subcommand};
use qobuz::{
    auth::Credentials,
    downloader::{Download, DownloadConfig, DownloadError, Progress},
    types::{extra::WithExtra, Album, Playlist, Track},
    ApiError,
};
use std::io::{self, Write};
use std::{fmt::Debug, path::Path};
use url::Url;

const QOBUZ_HOSTS: [&str; 2] = ["play.qobuz.com", "open.qobuz.com"];

#[derive(Parser, Clone, Debug)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Clone, Debug)]
enum Command {
    /// Download an item.
    Dl {
        /// The URL or favorite type of the item to download.
        url: String,
    },
}

async fn get_item(client: &qobuz::Client, url: Url) -> Result<Type, ApiError> {
    let Some(url::Host::Domain(domain)) = url.host() else {
        todo!();
    };
    if !QOBUZ_HOSTS.contains(&domain) {
        return todo!();
    }
    let mut path = url.path_segments().unwrap();
    let kind = path.next().unwrap();
    let id = path.next().unwrap();

    macro_rules! get {
        ($t:ident) => {
            Type::$t(client.get_item::<$t<WithExtra>>(id).await?)
        };
    }

    Ok(match kind {
        "track" => get!(Track),
        "album" => get!(Album),
        "playlist" => get!(Playlist),
        _ => todo!(),
    })
}

#[derive(Debug, Clone)]
enum Type {
    Track(Track<WithExtra>),
    Album(Album<WithExtra>),
    Playlist(Playlist<WithExtra>),
}

macro_rules! impl_all_variants {
    ($self:expr, $name:ident, $inner:expr) => {
        match $self {
            Self::Track($name) => $inner,
            Self::Album($name) => $inner,
            Self::Playlist($name) => $inner,
        }
    };
}

impl Type {
    async fn download(
        &self,
        download_config: &DownloadConfig,
        client: &qobuz::Client,
    ) -> Result<(), DownloadError> {
        impl_all_variants!(self, item, {
            download_item(item, download_config, client).await
        })
    }
}

async fn download_item<T: Download>(
    item: &T,
    download_config: &DownloadConfig,
    client: &qobuz::Client,
) -> Result<(), DownloadError> {
    let (fut, progress_rx) = item.download(download_config, client);
    tokio::spawn(async move {
        let mut rx = progress_rx.await.expect("No status returned");
        while rx.changed().await.is_ok() {
            let progress = rx.borrow();
            println!("{}%", progress.progress_percentage());
            io::stdout().flush().unwrap();
        }
    });
    fut.await.map(|_| ())
}

async fn make_client() -> qobuz::Client {
    qobuz::Client::new(Credentials::from_env().unwrap())
        .await
        .unwrap()
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();
    let download_config = DownloadConfig::builder(Path::new("music"))
        .overwrite(true)
        .build()
        .unwrap();
    match args.command {
        Command::Dl { url } => match url.as_str() {
            "tracks" | "track" => todo!(),
            "albums" | "album" => todo!(),
            "playlists" | "playlist" => todo!(),
            v => {
                let client = make_client().await;
                let url = v.parse().unwrap();
                let item = get_item(&client, url).await.unwrap();
                println!("{:?}", item);
                item.download(&download_config, &client).await.unwrap();
            }
        },
    }
}
