use clap::{Parser, Subcommand};
use qobuz::{
    auth::Credentials,
    downloader::{AutoRootDir, Download, DownloadConfig, DownloadError},
    types::{extra::WithExtra, Album, Playlist, Track},
    ApiError,
};
use std::fmt::Debug;
use std::io::{self, Write};
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
        todo!();
    }
    let mut path = url.path_segments().unwrap();
    let kind = path.next().unwrap();
    let id = path.next().unwrap();

    macro_rules! get {
        ($t:ident) => {
            Type::$t(Box::new(client.get_item::<$t<WithExtra>>(id).await?))
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
    Track(Box<Track<WithExtra>>),
    Album(Box<Album<WithExtra>>),
    Playlist(Box<Playlist<WithExtra>>),
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

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        impl_all_variants!(self, item, { std::fmt::Debug::fmt(&item, f) })
    }
}

impl Type {
    async fn download(
        &self,
        download_config: &DownloadConfig,
        client: &qobuz::Client,
    ) -> Result<(), DownloadError> {
        impl_all_variants!(self, item, {
            download_item(item.as_ref(), download_config, client).await
        })
    }
}

async fn download_item<T: Download + Sync>(
    item: &T,
    download_config: &DownloadConfig,
    client: &qobuz::Client,
) -> Result<(), DownloadError> {
    let (fut, progress_rx) = item.download(download_config, client);
    tokio::spawn(async move {
        let mut rx = progress_rx.await.expect("No status returned");
        while rx.changed().await.is_ok() {
            println!("{}%", *rx.borrow());
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
    let download_config = DownloadConfig::builder(AutoRootDir)
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
                println!("{item}");
                item.download(&download_config, &client).await.unwrap();
            }
        },
    }
}
