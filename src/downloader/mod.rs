use crate::{
    quality::{FileExtension, Quality},
    types::{
        extra::{ExtraFlag, WithExtra, WithoutExtra},
        traits::RootEntity,
        Album, Array, Playlist, QobuzType, Track,
    },
    ApiError,
};
use futures::{Future, StreamExt};
use std::{
    ffi::{OsStr, OsString},
    fmt::Debug,
    io::Write,
    path::PathBuf,
};
use thiserror::Error;
use tokio::{
    fs::OpenOptions,
    sync::{mpsc, oneshot, watch},
};
pub mod tagging;
use tagging::{tag_track, TaggingError};
pub mod path_format;
use path_format::PathFormat;

mod delayed_watch;
use delayed_watch::DelayedWatchReceiver;
#[macro_use]
mod builder;

builder! {
    /// Options for downloads.
    ///
    /// * `root_dir` and `m3u_dir` - Where tracks and playlists are saved. By default, `m3u_dir`
    /// will be set to `{root_dir}/playlists`.
    /// * `quality` - The quality at which tracks are downloaded.
    /// * `overwrite` - Whether or not to overwrite existing tracks and playlists.
    /// * `path_format` - The format options for file names.
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
    DownloadConfig {
        provided: {
            root_dir: PathBuf = impl Into<PathBuf> => root_dir.into(),
        },
        default: {
            m3u_dir: PathBuf = root_dir.join("playlists"),
            quality: Quality = Quality::default(),
            overwrite: bool = false,
            path_format: PathFormat = PathFormat::default(),
        }
    },
    verify: Result<(), NonExistentDirectoryError> = {
        if !root_dir.exists() {
            return Err(NonExistentDirectoryError::RootDir(root_dir));
        }
        if !m3u_dir.exists() {
            return Err(NonExistentDirectoryError::M3uDir(m3u_dir));
        }
        Ok(())
    }
}

pub trait Download: RootEntity {
    type ProgressType: Debug;
    type PathInfoType: Debug;

    #[must_use]
    fn download(
        &self,
        download_config: &DownloadConfig,
        client: &crate::Client,
    ) -> (
        impl Future<Output = Result<Self::PathInfoType, DownloadError>>,
        DelayedWatchReceiver<Self::ProgressType>,
    );
}

#[derive(Debug, Clone)]
pub struct TrackDownloadProgress {
    pub downloaded: u64,
    pub total: u64,
}

#[derive(Debug)]
pub struct ArrayDownloadProgress {
    pub current: Track<WithExtra>,
    pub position: usize,
    pub total: usize,
    pub track_progress_rx: oneshot::Receiver<watch::Receiver<TrackDownloadProgress>>,
}

impl Download for Track<WithExtra> {
    type ProgressType = TrackDownloadProgress;
    type PathInfoType = PathBuf;

    /// Download and tag a track, returning the download locations of the album and track.
    fn download(
        &self,
        download_config: &DownloadConfig,
        client: &crate::Client,
    ) -> (
        impl Future<Output = Result<Self::PathInfoType, DownloadError>>,
        DelayedWatchReceiver<Self::ProgressType>,
    ) {
        let (progress_tx, progress_rx) = delayed_watch::channel();

        let fut = async move {
            let album_path = self.album.get_path(download_config);
            std::fs::create_dir_all(&album_path)?;
            let path = self.get_path(download_config);

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
                        std::io::ErrorKind::AlreadyExists => Ok(path),
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

            Ok(path)
        };

        (fut, progress_rx)
    }
}

async fn download_tracks(
    tracks: &[Track<WithExtra>],
    download_config: &DownloadConfig,
    client: &crate::Client,
    progress_tx: mpsc::Sender<ArrayDownloadProgress>,
) -> Result<Vec<PathBuf>, DownloadError> {
    let mut paths: Vec<PathBuf> = vec![];
    for (i, track) in tracks.iter().enumerate() {
        let (fut, track_progress_rx) = track.download(download_config, client);

        progress_tx
            .send(ArrayDownloadProgress {
                current: track.clone(), // TODO: Avoid cloning track
                position: i,
                total: tracks.len(),
                track_progress_rx,
            })
            .await
            .expect("The mpsc will never be closed on the receiving side");

        let path = fut.await?;

        paths.push(path);
    }
    Ok(paths)
}

impl Download for Album<WithExtra> {
    type ProgressType = ArrayDownloadProgress;
    type PathInfoType = Vec<PathBuf>;

    fn download(
        &self,
        download_config: &DownloadConfig,
        client: &crate::Client,
    ) -> (
        impl Future<Output = Result<Self::PathInfoType, DownloadError>>,
        DelayedWatchReceiver<Self::ProgressType>,
    ) {
        let tracks = self.get_tracks_with_extra();
        let (progress_tx, progress_rx) = delayed_watch::channel();

        let fut =
            async move { download_tracks(&tracks, download_config, client, progress_tx).await };

        (fut, progress_rx)
    }
}

#[derive(Debug)]
pub struct PlaylistPathInfo {
    pub track_paths: Vec<PathBuf>,
    pub m3u_path: PathBuf,
}

impl Download for Playlist<WithExtra> {
    type ProgressType = ArrayDownloadProgress;
    type PathInfoType = PlaylistPathInfo;

    fn download(
        &self,
        download_config: &DownloadConfig,
        client: &crate::Client,
    ) -> (
        impl Future<Output = Result<Self::PathInfoType, DownloadError>>,
        DelayedWatchReceiver<Self::ProgressType>,
    ) {
        let tracks = &self.tracks.items;

        let (progress_tx, progress_rx) = delayed_watch::channel();

        let fut = async move {
            download_tracks(tracks, download_config, client, progress_tx)
                .await
                .and_then(|track_paths| {
                    let m3u_path = write_m3u(self, download_config)?;
                    Ok(PlaylistPathInfo {
                        track_paths,
                        m3u_path,
                    })
                })
        };

        (fut, progress_rx)
    }
}

pub trait GetPath: QobuzType {
    fn get_path(&self, download_config: &DownloadConfig) -> PathBuf;
}

impl GetPath for Track<WithExtra> {
    fn get_path(&self, download_config: &DownloadConfig) -> PathBuf {
        self.album.get_path(download_config).join(format!(
            "{}.{}",
            sanitize_filename(&download_config.path_format.get_track_file_basename(self)),
            FileExtension::from(&download_config.quality)
        ))
    }
}

impl<EF> GetPath for Album<EF>
where
    EF: ExtraFlag<Array<Track<WithoutExtra>>>,
{
    fn get_path(&self, download_config: &DownloadConfig) -> PathBuf {
        download_config.root_dir.join(sanitize_filename(
            &download_config
                .path_format
                .get_album_dir(self, &download_config.quality),
        ))
    }
}

impl<EF> GetPath for Playlist<EF>
where
    EF: ExtraFlag<Array<Track<WithExtra>>>,
{
    fn get_path(&self, download_config: &DownloadConfig) -> PathBuf {
        download_config
            .m3u_dir
            .join(format!("{}.m3u", sanitize_filename(&self.name)))
    }
}

/// Writes the m3u file for the specified playlist, without downloading the tracks.
#[allow(clippy::missing_panics_doc)]
pub fn write_m3u(
    playlist: &Playlist<WithExtra>,
    download_config: &DownloadConfig,
) -> Result<PathBuf, std::io::Error> {
    let m3u_path = playlist.get_path(download_config);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .create_new(!download_config.overwrite) // (Shadows create and truncate)
        .open(&m3u_path)?;
    let track_paths = playlist
        .tracks
        .items
        .iter()
        .map(|p| {
            p.get_path(download_config)
                .strip_prefix(&download_config.root_dir)
                .expect("Path is relative to root")
                .as_os_str()
                .to_owned()
        })
        .collect::<Vec<OsString>>();
    let track_paths = track_paths.join(
        // SAFETY: \n is a valid UTF-8 character
        // > Callers must pass in bytes that originated as a mixture of validated UTF-8 and [...]
        unsafe { OsStr::from_encoded_bytes_unchecked(b"\n") },
    );
    file.write_all(track_paths.as_encoded_bytes())?;

    Ok(m3u_path)
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
}

#[derive(Debug, Error)]
pub enum NonExistentDirectoryError {
    #[error("Non existent download root directory `{0}`")]
    RootDir(PathBuf),
    #[error("Non existent m3u directory `{0}`")]
    M3uDir(PathBuf),
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
        let (fut, progress_rx) = track.download(&download_config, &client);
        fut.await.unwrap();
        let final_progress = progress_rx.await.unwrap().borrow().clone();
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
        let (fut, progress_rx) = album.download(&download_config, &client);
        fut.await.unwrap();
        let rx = progress_rx.await.unwrap();
        let (final_position, total) = {
            let final_progress = rx.borrow();
            (final_progress.position, final_progress.total)
        };
        assert!(final_position == total - 1);
    }
}
