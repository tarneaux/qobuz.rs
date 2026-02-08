use crate::{
    quality::{FileExtension, Quality},
    runtime_formatter::{Format, Formattable, IllegalPlaceholderError, Placeholder},
    types::{
        extra::WithExtra,
        formattable::{AlbumPlaceholder, TrackPlaceholder},
        traits::RootEntity,
        Album, AlbumExtra, Playlist, PlaylistExtra, QobuzType, Track,
    },
    ApiError,
};
use futures::{Future, StreamExt};
use std::fmt::Display;
use std::{
    ffi::{OsStr, OsString},
    fmt::Debug,
    io::Write,
    path::PathBuf,
    str::FromStr,
};
use thiserror::Error;
use tokio::{
    fs::OpenOptions,
    sync::{mpsc, oneshot, watch},
};

pub mod tagging;
use tagging::{tag_track, TaggingError};

mod delayed_watch;
use delayed_watch::DelayedWatchReceiver;
#[macro_use]
mod builder;

pub const DEFAULT_ALBUM_DIR_NAME_FORMAT: &str = "{artist} - {title} ({year}) [{quality}]";
pub const DEFAULT_TRACK_FILE_NAME_FORMAT: &str = "{media_number}-{track_number}. {title}";

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
    ///     downloader::DownloadConfig,
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
            overwrite_playlists: bool = true,
            track_file_name_format: Format<DownloadedItemPlaceholder<TrackPlaceholder>> = DEFAULT_TRACK_FILE_NAME_FORMAT.parse().expect("Default format is correct"),
            album_dir_name_format: Format<DownloadedItemPlaceholder<AlbumPlaceholder>> = DEFAULT_ALBUM_DIR_NAME_FORMAT.parse().expect("Default format is correct"),
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
    type ProgressType: Progress + Send + Sync + 'static;
    type PathInfoType: Debug + Send + Sync + 'static;

    #[must_use]
    fn download(
        &self,
        download_config: &DownloadConfig,
        client: &crate::Client,
    ) -> (
        impl Future<Output = Result<Self::PathInfoType, DownloadError>> + Send,
        DelayedWatchReceiver<Self::ProgressType>,
    );
}

pub trait Progress: Debug + Display {
    /// The numerator of the progress.
    fn progress_numerator(&self) -> u64;
    /// The denominator of the progress.
    fn progress_denominator(&self) -> u64;
    /// The progress as an integer percentage.
    fn progress_percentage(&self) -> u8 {
        (self.progress_numerator() * 100 / self.progress_denominator())
            .try_into()
            .expect("Percentage should fit in u8")
    }
}

#[derive(Debug, Clone)]
pub struct TrackDownloadProgress {
    pub downloaded: u64,
    pub total: u64,
}

impl Progress for TrackDownloadProgress {
    fn progress_numerator(&self) -> u64 {
        self.downloaded
    }
    fn progress_denominator(&self) -> u64 {
        self.total
    }
}

impl std::fmt::Display for TrackDownloadProgress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "Track download progress: {}%",
            self.progress_percentage()
        )
    }
}

/// The progress of an array download.
#[derive(Debug)]
pub struct ArrayDownloadProgress {
    /// The item that will be downloaded now.
    pub current_item: Track<WithExtra>,
    /// The index of the item that will be downloaded now (`current_index` < `total`).
    pub current_index: usize,
    /// The total count of items in the download array.
    pub total: usize,
    /// The progress receiver of the item that is being downloaded.
    pub track_progress_rx: oneshot::Receiver<watch::Receiver<TrackDownloadProgress>>,
}

impl ArrayDownloadProgress {
    const fn downloaded(&self) -> usize {
        self.current_index - 1
    }
}

impl Progress for ArrayDownloadProgress {
    fn progress_numerator(&self) -> u64 {
        self.downloaded() as u64
    }
    fn progress_denominator(&self) -> u64 {
        self.total as u64
    }
}

impl std::fmt::Display for ArrayDownloadProgress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "Array download progress: {}%",
            self.progress_percentage()
        )
    }
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

            if !download_config.overwrite && path.try_exists()? {
                return Ok(path);
            }

            let tmp_file_name = {
                let mut s = path
                    .file_stem()
                    .expect("File name is nonempty")
                    .to_os_string();
                s.push(OsString::from(".tmp."));
                s.push(path.extension().expect("Extension is nonempty"));
                s
            };

            let tmp_path = path.with_file_name(tmp_file_name);

            let mut out = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&tmp_path)
                .await?;
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

            tag_track(self, &tmp_path, &self.album).await?;

            std::fs::rename(&tmp_path, &path)?;

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
                current_item: track.clone(), // TODO: Avoid cloning track
                current_index: i,
                total: tracks.len(),
                track_progress_rx,
            })
            .await
            .expect("The mpsc will never be closed on the receiving side");

        println!("{track}");

        match fut.await {
            Err(DownloadError::ApiError(ApiError::IsSample)) => {
                continue;
            }
            Err(e) => {
                return Err(e);
            }
            Ok(path) => paths.push(path),
        };
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
                    let m3u_path =
                        write_m3u(self, download_config).map_err(DownloadError::M3uWritingError)?;
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
            sanitize_filename(
                &DownloadedItem {
                    inner: self,
                    quality: &download_config.quality
                }
                .format(&download_config.track_path_format,)
            ),
            FileExtension::from(&download_config.quality)
        ))
    }
}

impl<EF> GetPath for Album<EF>
where
    EF: AlbumExtra,
{
    fn get_path(&self, download_config: &DownloadConfig) -> PathBuf {
        download_config.root_dir.join(sanitize_filename(
            &DownloadedItem {
                inner: self,
                quality: &download_config.quality,
            }
            .format(&download_config.album_path_format),
        ))
    }
}

impl<EF: PlaylistExtra> GetPath for Playlist<EF> {
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
        .create_new(!download_config.overwrite_playlists) // (Shadows create and truncate)
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
    #[error("IO error while writing M3u file `{0}`")]
    M3uWritingError(std::io::Error),
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
    let filename = filename
        .trim()
        .replace('/', "-")
        .replace(|c: char| !c.is_alphanumeric(), "_");
    filename.trim_start_matches('.').to_string()
}

/// Automatically get the root dir from environment or a default value.
///
/// If used multiple times, should probably be converted to a PathBuf ahead of time for optimal
/// performance, but the behavior won't change
pub struct AutoRootDir;

impl From<AutoRootDir> for PathBuf {
    fn from(_: AutoRootDir) -> Self {
        match std::env::var("QOBUZ_DL_ROOT") {
            Ok(v) => v.into(),
            Err(e) => {
                match e {
                    std::env::VarError::NotPresent => {}
                    std::env::VarError::NotUnicode(_) => {
                        println!(
                            "WARNING: Your QOBUZ_DL_ROOT variable couldn't be decoded as unicode. Using default."
                        );
                    }
                }
                std::env::home_dir()
                    .expect("Couldn't get home dir")
                    .join("Music")
            }
        }
    }
}

pub struct DownloadedItem<'a, T: Formattable> {
    pub inner: &'a T,
    pub quality: &'a Quality,
}

impl<'a, T: Formattable> Formattable for DownloadedItem<'a, T> {
    type Placeholder = DownloadedItemPlaceholder<T::Placeholder>;

    fn get_field(&self, field: &Self::Placeholder) -> String {
        match field {
            DownloadedItemPlaceholder::Inner(v) => self.inner.get_field(v),
            DownloadedItemPlaceholder::Quality => self.quality.to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DownloadedItemPlaceholder<T: Placeholder> {
    Inner(T),
    Quality,
}

impl<T: Placeholder> FromStr for DownloadedItemPlaceholder<T> {
    type Err = IllegalPlaceholderError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "quality" => Ok(Self::Quality),
            v => T::from_str(v).map(|v| Self::Inner(v)),
        }
    }
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

        let new_download_config = download_config.rebuild().overwrite(false).build().unwrap();
        let (fut, progress_rx) = track.download(&new_download_config, &client);
        fut.await.unwrap();
        assert!(progress_rx.await.is_err());
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
            (final_progress.current_index, final_progress.total)
        };
        assert!(final_position == total - 1);
    }
}
