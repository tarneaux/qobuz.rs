use crate::{
    quality::{FileExtension, Quality},
    types::{
        extra::{ExtraFlag, WithExtra, WithoutExtra},
        traits::RootEntity,
        Album, Array, Playlist, Track,
    },
    ApiError,
};
use futures::{Future, StreamExt};
use std::{
    ffi::OsStr,
    fmt::Debug,
    io::Write,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};
use thiserror::Error;
use tokio::{
    fs::OpenOptions,
    sync::{oneshot, watch},
};
pub mod tagging;
use tagging::{tag_track, TaggingError};
pub mod path_format;
use path_format::PathFormat;

mod delayed_watch;
#[macro_use]
mod builder;

/// Options for downloads.
///
/// * `client` - Will be used to query information and download URLs.
/// * `root` - Download root directory.
/// * `m3u_dir` - Directory where to put m3u files.
/// * `quality` - Quality to download at.
/// * `overwrite` - Whether to overwrite existing files.
/// * `path_format` - The path format for tracks and albums
///
/// # Example
///
/// ```
/// # use tokio_test;
/// # tokio_test::block_on(async {
/// use qobuz::{
///     auth::Credentials,
///     Client,
///     downloader::{DownloadConfig, path_format::PathFormat},
///     quality::Quality
/// };
/// use std::path::Path;
/// let credentials = Credentials::from_env().unwrap();
/// let client = Client::new(credentials).await.unwrap();
/// let opts = DownloadConfig::builder(Path::new("music"))
///     .quality(Quality::Mp3)
///     .overwrite(true)
///     .build()
///     .unwrap();
/// # })
/// ```
#[derive(Debug, Clone)]
pub struct DownloadConfig {
    root_dir: Box<Path>,
    m3u_dir: Box<Path>,
    quality: Quality,
    overwrite: bool,
    path_format: PathFormat,
}

impl DownloadConfig {
    pub fn builder(root_dir: impl Into<Box<Path>>) -> DownloadConfigBuilder {
        DownloadConfigBuilder::new(root_dir.into())
    }

    #[must_use]
    pub fn rebuild(self) -> DownloadConfigBuilder {
        self.into()
    }

    /// Write an M3U file for a playlist with a certain `name`, containing the already downloaded
    /// tracks `track_paths`, returning the new M3U file's path.
    pub fn write_m3u(
        &self,
        playlist: &Playlist<WithExtra>,
        track_paths: &[PathBuf],
    ) -> Result<PathBuf, DownloadError> {
        let m3u_path = self.get_m3u_path(playlist);
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .create_new(!self.overwrite) // (Shadows create and truncate)
            .open(&m3u_path)?;
        let track_paths = track_paths
            .iter()
            .map(|p| Ok(p.strip_prefix(&self.root_dir)?.as_os_str()))
            .collect::<Result<Vec<&OsStr>, std::path::StripPrefixError>>()?;
        let track_paths = track_paths.join(OsStr::from_bytes(b"\n"));
        file.write_all(track_paths.as_encoded_bytes())?;

        Ok(m3u_path)
    }

    pub fn get_album_path<EF>(&self, album: &Album<EF>) -> PathBuf
    where
        EF: ExtraFlag<Array<Track<WithoutExtra>>>,
    {
        let mut path = self.root_dir.to_path_buf();
        path.push(sanitize_filename(
            &self.path_format.get_album_dir(album, &self.quality),
        ));
        path
    }

    pub fn get_track_path<EF>(&self, track: &Track<EF>, album_path: &Path) -> PathBuf
    where
        EF: ExtraFlag<Album<WithoutExtra>>,
    {
        let mut path = album_path.to_path_buf();
        path.push(format!(
            "{}.{}",
            sanitize_filename(&self.path_format.get_track_file_basename(track)),
            FileExtension::from(&self.quality)
        ));
        path
    }

    #[must_use]
    pub fn get_m3u_path(&self, playlist: &Playlist<WithExtra>) -> PathBuf {
        let mut path = self.m3u_dir.to_path_buf();
        path.push(format!("{}.m3u", sanitize_filename(&playlist.name)));
        path
    }
}

pub trait Download: RootEntity {
    type ProgressType: Debug + Clone;

    fn download(
        &self,
        download_config: &DownloadConfig,
        client: &crate::Client,
    ) -> (
        impl Future<Output = Result<(), DownloadError>>,
        DownloadInfo<Self::ProgressType>,
    );
}

pub struct DownloadInfo<ProgressType> {
    pub path: PathBuf,
    pub progress_rx: oneshot::Receiver<Option<watch::Receiver<ProgressType>>>,
}

#[derive(Debug, Clone)]
pub struct TrackDownloadProgress {
    pub downloaded: u64,
    pub total: u64,
}

#[derive(Debug, Clone)]
pub struct ArrayDownloadProgress {
    pub current: Track<WithExtra>,
    pub position: usize,
    pub total: usize,
    pub track_path: PathBuf,
}

impl Download for Track<WithExtra> {
    type ProgressType = TrackDownloadProgress;

    /// Download and tag a track, returning the download locations of the album and track.
    fn download(
        &self,
        download_config: &DownloadConfig,
        client: &crate::Client,
    ) -> (
        impl Future<Output = Result<(), DownloadError>>,
        DownloadInfo<Self::ProgressType>,
    ) {
        let album_path = download_config.get_album_path(&self.album);
        let path = download_config.get_track_path(self, &album_path);

        let (progress_tx, progress_rx) = delayed_watch::channel();

        let fut = {
            let path = path.clone();
            async move {
                std::fs::create_dir_all(&album_path)?;

                let mut out = match OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .create_new(!download_config.overwrite) // (Shadows create and truncate)
                    .open(&path)
                    .await
                {
                    Ok(v) => v,
                    Err(e) => {
                        return match e.kind() {
                            // TODO: Remove when using temp files
                            std::io::ErrorKind::AlreadyExists => Ok(()),
                            _ => Err(DownloadError::IoError(e)),
                        };
                    }
                };
                let (mut bytes_stream, content_length) = client
                    .stream_track(&self.id.to_string(), download_config.quality.clone())
                    .await?;
                let mut downloaded: u64 = 0;
                while let Some(item) = bytes_stream.next().await {
                    let item = item?;
                    tokio::io::copy(&mut item.as_ref(), &mut out).await?;
                    downloaded += item.len() as u64;
                    progress_tx
                        .send(TrackDownloadProgress {
                            downloaded,
                            total: content_length,
                        })
                        .await
                        .expect("The mpsc will never be closed on the receiving side");
                }

                tag_track(self, &path, &self.album).await?;

                Ok(())
            }
        };

        (fut, DownloadInfo { path, progress_rx })
    }
}

impl Download for Album<WithExtra> {
    type ProgressType = ArrayDownloadProgress;

    fn download(
        &self,
        download_config: &DownloadConfig,
        client: &crate::Client,
    ) -> (
        impl Future<Output = Result<(), DownloadError>>,
        DownloadInfo<Self::ProgressType>,
    ) {
        let tracks = self.get_tracks_with_extra();

        let (progress_tx, progress_rx) = delayed_watch::channel();

        let fut = async move {
            for (i, track) in tracks.iter().enumerate() {
                let (fut, res) = track.download(download_config, client);

                progress_tx
                    .send(ArrayDownloadProgress {
                        current: track.clone(), // TODO: Avoid cloning track
                        position: i,
                        total: tracks.len(),
                        track_path: res.path,
                    })
                    .await
                    .expect("The mpsc will never be closed on the receiving side");

                fut.await?;
            }
            Ok(())
        };

        let path = download_config.get_album_path(self);
        (fut, DownloadInfo { path, progress_rx })
    }
}

// TODO: Deduplicate implementations
impl Download for Playlist<WithExtra> {
    type ProgressType = ArrayDownloadProgress;

    fn download(
        &self,
        download_config: &DownloadConfig,
        client: &crate::Client,
    ) -> (
        impl Future<Output = Result<(), DownloadError>>,
        DownloadInfo<Self::ProgressType>,
    ) {
        let tracks = &self.tracks.items;

        let (progress_tx, progress_rx) = delayed_watch::channel();

        let fut = async move {
            let mut track_paths: Vec<PathBuf> = vec![];
            for (i, track) in tracks.iter().enumerate() {
                let (fut, res) = track.download(download_config, client);

                progress_tx
                    .send(ArrayDownloadProgress {
                        current: track.clone(), // TODO: Avoid cloning
                        position: i,
                        total: tracks.len(),
                        track_path: res.path.clone(),
                    })
                    .await
                    .expect("The mpsc will never be closed on the receiving side");

                fut.await?;
                track_paths.push(res.path);
            }
            download_config.write_m3u(self, &track_paths)?;
            Ok(())
        };

        let path = download_config.get_m3u_path(self);
        (fut, DownloadInfo { path, progress_rx })
    }
}

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("tagging error `{0}`")]
    TaggingError(#[from] TaggingError),
    #[error("IO error `{0}`")]
    IoError(#[from] std::io::Error),
    #[error("reqwest error `{0}`")]
    ReqwestError(#[from] reqwest::Error),
    #[error("API error `{0}`")]
    ApiError(#[from] ApiError),
    #[error("Failed to strip prefix from path: `{0}`")]
    PathStripPrefixError(#[from] std::path::StripPrefixError),
}

builder! {
    /// A builder for `DownloadConfig`.
    ///
    /// * `root_dir` and `m3u_dir` - Where tracks and playlists are saved. By default, `m3u_dir`
    /// will be set to `{root_dir}/playlists`.
    /// * `quality` - The quality at which tracks are downloaded.
    /// * `overwrite` - Whether or not to overwrite existing tracks and playlists.
    /// * `path_format` - The format options for file names.
    DownloadConfigBuilder,
    DownloadConfig,
    {
        required: {
            root_dir: Box<Path>,
        },
        default: {
            m3u_dir: Box<Path> = root_dir.to_path_buf().join("playlists").into(),
            quality: Quality = Quality::default(),
            overwrite: bool = false,
            path_format: PathFormat = PathFormat::default(),
        }
    },
    {
        if !root_dir.exists() {
            return Err(NonExistentDirectoryError::RootDir(root_dir));
        }
        if !m3u_dir.exists() {
            return Err(NonExistentDirectoryError::M3uDir(m3u_dir));
        }
        Ok(())
    },
    NonExistentDirectoryError
}

#[derive(Debug, Error)]
pub enum NonExistentDirectoryError {
    #[error("Non existent download root directory `{0}`")]
    RootDir(Box<Path>),
    #[error("Non existent m3u directory `{0}`")]
    M3uDir(Box<Path>),
}

#[must_use]
pub fn sanitize_filename(filename: &str) -> String {
    let filename = filename.trim().replace('/', "-");
    filename.trim_start_matches('.').to_string()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::test_utils::{make_client, make_download_config};
    use tokio::test;

    const HIRES192_TRACK: &str = "18893849"; // Creedence Clearwater Revival - Lodi

    #[test]
    async fn test_download_track() {
        let (client, download_config) = (make_client().await, make_download_config());
        let track = client.get_track(HIRES192_TRACK).await.unwrap();
        let (fut, res) = track.download(&download_config, &client);
        fut.await.unwrap();
        let final_progress = res.progress_rx.await.unwrap().unwrap().borrow().clone();
        assert!(final_progress.downloaded == final_progress.total);
    }

    #[test]
    async fn test_download_album() {
        let (client, download_config) = (make_client().await, make_download_config());
        let album = client
            .get_album("lz75qrx8pnjac")
            .await
            .map_err(|e| {
                println!("{e:?}");
                e
            })
            .unwrap();
        let (fut, res) = album.download(&download_config, &client);
        fut.await.unwrap();
        let final_progress = res.progress_rx.await.unwrap().unwrap().borrow().clone();
        assert!(final_progress.position == final_progress.total - 1);
    }
}
