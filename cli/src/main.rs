use clap::{Parser, Subcommand};
use qobuz::{
    auth::{Credentials, LoginError},
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

async fn get_item(client: &qobuz::Client, url: Url) -> Result<Type, GetItemError> {
    let Some(url::Host::Domain(domain)) = url.host() else {
        return Err(GetItemError::NoDomain);
    };
    if !QOBUZ_HOSTS.contains(&domain) {
        return Err(GetItemError::NotAQobuzUrl);
    }
    let mut path = url.path_segments().ok_or(GetItemError::PathErr)?;
    let kind = path.next().ok_or(GetItemError::PathErr)?;
    let id = path.next().ok_or(GetItemError::PathErr)?;

    macro_rules! get {
        ($t:ident) => {
            Type::$t(Box::new(client.get_item::<$t<WithExtra>>(id).await?))
        };
    }

    match kind {
        "track" => Ok(get!(Track)),
        "album" => Ok(get!(Album)),
        "playlist" => Ok(get!(Playlist)),
        e => Err(GetItemError::UnrecognizedKind(e.to_string())),
    }
}

#[derive(Debug)]
enum GetItemError {
    ApiError(ApiError),
    UnrecognizedKind(String),
    NotAQobuzUrl,
    NoDomain,
    PathErr,
}

impl From<ApiError> for GetItemError {
    fn from(v: ApiError) -> Self {
        Self::ApiError(v)
    }
}

impl std::fmt::Display for GetItemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Self::ApiError(e) => write!(f, "API error: {e}"),
            Self::UnrecognizedKind(kind) => write!(f, "Unrecognized kind of data: {kind}"),
            Self::NotAQobuzUrl => write!(f, "Supplied URL is not a Qobuz URL"),
            Self::NoDomain => write!(f, "Couldn't get URL host domain"),
            Self::PathErr => write!(f, "Error while getting URL path parts"),
        }
    }
}

impl std::error::Error for GetItemError {}

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
            println!("{}", *rx.borrow());
            let _ = io::stdout().flush();
        }
    });
    fut.await.map(|_| ())
}

async fn make_client() -> Result<qobuz::Client, LoginError> {
    qobuz::Client::new(Credentials::from_env().expect("Couldn't get credentials from environment"))
        .await
}

macro_rules! fatal {
    ($ec:literal, $t:literal) => {{
        println!($t);
        std::process::exit($ec);
    }};
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();
    let download_config = DownloadConfig::builder(AutoRootDir)
        .overwrite(true)
        .build()
        .unwrap_or_else(|e| fatal!(2, "Error while building downloader: {e}"));
    match args.command {
        Command::Dl { url } => match url.as_str() {
            "tracks" | "track" => todo!(),
            "albums" | "album" => todo!(),
            "playlists" | "playlist" => todo!(),
            v => {
                let client = make_client()
                    .await
                    .unwrap_or_else(|e| fatal!(1, "Couldn't login to Qobuz: {e}"));
                let url: Url = v
                    .parse()
                    .unwrap_or_else(|e| fatal!(2, "Couldn't parse URL {v}: {e}"));
                let item = get_item(&client, url.clone())
                    .await
                    .unwrap_or_else(|e| fatal!(1, "Error while getting item {url}: {e}"));
                item.download(&download_config, &client)
                    .await
                    .unwrap_or_else(|e| fatal!(1, "Couldn't download item {url}: {e}"));
            }
        },
    }
}
