use crate::{
    quality::{FileExtension, Quality},
    types::{
        extra::{ExtraFlag, RootEntity, WithExtra, WithoutExtra},
        Album, Array, Playlist, Track,
    },
    ApiError,
};
use futures::{Future, StreamExt};
use std::{
    error::Error,
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

#[derive(Debug, Clone)]
pub struct Downloader {
    client: crate::Client,
    root: Box<Path>,
    m3u_dir: Box<Path>,
    quality: Quality,
    overwrite: bool,
    path_format: PathFormat,
}

impl Downloader {
    /// Create a new `Downloader` which will use the given `client` to download to the given
    /// `root`, putting m3u playlist files in `m3u_dir`.
    ///
    /// # Example
    ///
    /// ```
    /// # use tokio_test;
    /// # tokio_test::block_on(async {
    /// use qobuz::{auth::Credentials, Client, downloader::Downloader};
    /// use std::path::Path;
    /// let credentials = Credentials::from_env().unwrap();
    /// let client = Client::new(credentials).await.unwrap();
    /// let downloader = Downloader::new(
    ///     client,
    ///     Path::new("music"),
    ///     Path::new("music/playlists")
    /// ).unwrap();
    /// # })
    /// ```
    pub fn new(
        client: crate::Client,
        root: &Path,
        m3u_dir: &Path,
        quality: Quality,
        overwrite: bool,
        path_format: PathFormat,
    ) -> Result<Self, NonExistentDirectoryError> {
        let root: Box<Path> = root.into();
        let m3u_dir: Box<Path> = m3u_dir.into();
        if !root.is_dir() {
            return Err(NonExistentDirectoryError::Root(root));
        }
        if !m3u_dir.is_dir() {
            return Err(NonExistentDirectoryError::M3uDir(m3u_dir));
        }
        Ok(Self {
            client,
            root,
            m3u_dir,
            quality,
            overwrite,
            path_format,
        })
    }

    /// Write an M3U file for a playlist with a certain `name`, containing the already downloaded
    /// tracks `track_paths`, returning the new M3U file's path.
    pub fn write_m3u(
        &self,
        playlist: &Playlist<WithExtra>,
        track_paths: &[PathBuf],
    ) -> Result<PathBuf, DownloadError> {
        let m3u_path = self.get_m3u_path(playlist)?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&m3u_path)?;
        let track_paths: Vec<&OsStr> = track_paths
            .iter()
            .map(|p| {
                p.strip_prefix(&self.root)
                    .expect("Path should be relative to music directory")
                    .as_os_str()
            })
            .collect();
        let track_paths = track_paths.join(OsStr::from_bytes(b"\n"));
        file.write_all(track_paths.as_encoded_bytes())?;

        Ok(m3u_path)
    }

    async fn download_track<EF>(
        &self,
        track: &Track<EF>,
        path: &Path,
        oneshot_tx: oneshot::Sender<watch::Receiver<TrackDownloadProgress>>,
    ) -> Result<(), DownloadError>
    where
        EF: ExtraFlag<Album<WithoutExtra>>,
        for<'a> &'a Track<EF>: Send,
    {
        let mut out = match OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .create_new(!self.overwrite) // (Shadows create and truncate)
            .open(&path)
            .await // TODO: Is async better than sync (isn't sync faster ?)
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
        let (mut bytes_stream, content_length) = self
            .client
            .stream_track(&track.id.to_string(), self.quality.clone())
            .await?;
        let mut downloaded: u64 = 0;
        let (tx, rx) = watch::channel(TrackDownloadProgress {
            downloaded,
            total: content_length,
        });
        let _ = oneshot_tx.send(rx);
        while let Some(item) = bytes_stream.next().await {
            let item = item?;
            tokio::io::copy(&mut item.as_ref(), &mut out).await?;
            downloaded += item.len() as u64;
            tx.send_replace(TrackDownloadProgress {
                downloaded,
                total: content_length,
            });
        }
        Ok(())
    }

    pub fn get_album_path<EF>(
        &self,
        album: &Album<EF>,
        ensure_exists: bool,
    ) -> Result<PathBuf, tera::Error>
    where
        EF: ExtraFlag<Array<Track<WithoutExtra>>>,
    {
        let mut path = self.root.to_path_buf();
        path.push(sanitize_filename(&self.path_format.get_album_dir(album)?));
        if ensure_exists && !path.is_dir() {
            std::fs::create_dir_all(&path)?;
        }
        Ok(path)
    }

    pub fn get_track_path<EF>(
        &self,
        track: &Track<EF>,
        album_path: &Path,
    ) -> Result<PathBuf, tera::Error>
    where
        EF: ExtraFlag<Album<WithoutExtra>>,
    {
        let mut path = album_path.to_path_buf();
        path.push(format!(
            "{}.{}",
            sanitize_filename(&self.path_format.get_track_file_basename(track)?,),
            FileExtension::from(&self.quality)
        ));
        Ok(path)
    }

    pub fn get_m3u_path(&self, playlist: &Playlist<WithExtra>) -> Result<PathBuf, tera::Error> {
        let mut path = self.m3u_dir.to_path_buf();
        path.push(format!(
            "{}.m3u",
            sanitize_filename(&self.path_format.get_m3u_file_basename(playlist)?),
        ));
        Ok(path)
    }
}

pub trait Download: RootEntity {
    type ProgressType: Debug + Clone;
    type E: Error;

    fn download(
        &self,
        downloader: &Downloader,
    ) -> Result<
        (
            impl Future<Output = Result<(), DownloadError>>,
            DownloadReturned<Self::ProgressType>,
        ),
        Self::E,
    >;
}

pub struct DownloadReturned<ProgressType> {
    pub path: PathBuf,
    pub rx: oneshot::Receiver<watch::Receiver<ProgressType>>,
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
    type E = tera::Error;

    /// Download and tag a track, returning the download locations of the album and track.
    fn download(
        &self,
        downloader: &Downloader,
    ) -> Result<
        (
            impl Future<Output = Result<(), DownloadError>>,
            DownloadReturned<Self::ProgressType>,
        ),
        Self::E,
    > {
        let album_path = downloader.get_album_path(&self.album, true)?;
        let path = downloader.get_track_path(self, &album_path)?;

        let (oneshot_tx, oneshot_rx) = oneshot::channel();

        let fut = {
            let path = path.clone();
            async move {
                downloader.download_track(self, &path, oneshot_tx).await?;
                tag_track(self, &path, &self.album).await?;

                Ok(())
            }
        };

        Ok((
            fut,
            DownloadReturned {
                path,
                rx: oneshot_rx,
            },
        ))
    }
}

impl Download for Album<WithExtra> {
    type ProgressType = ArrayDownloadProgress;
    type E = tera::Error;

    fn download(
        &self,
        downloader: &Downloader,
    ) -> Result<
        (
            impl Future<Output = Result<(), DownloadError>>,
            DownloadReturned<Self::ProgressType>,
        ),
        Self::E,
    > {
        let tracks = &self.tracks.items;

        let (oneshot_tx, oneshot_rx) = oneshot::channel();

        let fut = async move {
            let mut iterator = tracks.iter().enumerate();

            let Some((i, first_track)) = iterator.next() else {
                // TODO: Is this the right way to handle this ?
                return Ok(());
            };

            // TODO: Make Track<WithExtra> without the redundant API query
            let track = downloader
                .client
                .get_track(&first_track.id.to_string())
                .await?;
            let (fut, res) = track.download(downloader)?;
            let (tx, rx) = watch::channel(ArrayDownloadProgress {
                current: track.clone(), // TODO: Avoid cloning track
                position: i,
                total: tracks.len(),
                track_path: res.path,
            });
            let _ = oneshot_tx.send(rx);

            fut.await?;

            for (i, track) in iterator {
                // TODO: Make Track<WithExtra> without the redundant API query
                let track = downloader.client.get_track(&track.id.to_string()).await?;
                let (fut, res) = track.download(downloader)?;

                tx.send_replace(ArrayDownloadProgress {
                    current: track.clone(), // TODO: Avoid cloning track
                    position: i,
                    total: tracks.len(),
                    track_path: res.path,
                });

                fut.await?;
            }
            Ok(())
        };

        let path = downloader.get_album_path(self, true)?;
        Ok((
            fut,
            DownloadReturned {
                path,
                rx: oneshot_rx,
            },
        ))
    }
}

// TODO: Deduplicate implementations
impl Download for Playlist<WithExtra> {
    type ProgressType = ArrayDownloadProgress;
    type E = DownloadError;

    fn download(
        &self,
        downloader: &Downloader,
    ) -> Result<
        (
            impl Future<Output = Result<(), DownloadError>>,
            DownloadReturned<Self::ProgressType>,
        ),
        Self::E,
    > {
        let tracks = &self.tracks.items;

        let (oneshot_tx, oneshot_rx) = oneshot::channel();

        let fut = async move {
            let mut iterator = tracks.iter().enumerate();

            let Some((i, first_track)) = iterator.next() else {
                // TODO: Is this the right way to handle this ?
                return Ok(());
            };

            let (fut, res) = first_track.download(downloader)?;
            let (tx, rx) = watch::channel(ArrayDownloadProgress {
                current: first_track.clone(), // TODO: Avoid cloning track
                position: i,
                total: tracks.len(),
                track_path: res.path,
            });
            let _ = oneshot_tx.send(rx);

            fut.await?;

            for (i, track) in iterator {
                // TODO: Make Track<WithExtra> without the redundant API query
                let track = downloader.client.get_track(&track.id.to_string()).await?;
                let (fut, res) = track.download(downloader)?;

                tx.send_replace(ArrayDownloadProgress {
                    current: track.clone(), // TODO: Avoid cloning track
                    position: i,
                    total: tracks.len(),
                    track_path: res.path,
                });

                fut.await?;
            }
            Ok(())
        };

        let track_paths = tracks
            .iter()
            .map(|t| -> Result<PathBuf, tera::Error> {
                let album_path = downloader.get_album_path(&t.album, true)?;
                downloader.get_track_path(t, &album_path)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let path = downloader.write_m3u(self, &track_paths)?;
        Ok((
            fut,
            DownloadReturned {
                path,
                rx: oneshot_rx,
            },
        ))
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
    #[error("Tera error `{0}`")]
    TeraError(#[from] tera::Error),
}

#[derive(Debug, Error)]
pub enum NonExistentDirectoryError {
    #[error("Non existent root download directory `{0}`")]
    Root(Box<Path>),
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
    use crate::test_utils::make_client_and_downloader;
    use tokio::test;

    const HIRES192_TRACK: &str = "18893849"; // Creedence Clearwater Revival - Lodi

    #[test]
    async fn test_download_track() {
        let (client, downloader) = make_client_and_downloader().await;
        let track = client.get_track(HIRES192_TRACK).await.unwrap();
        let (fut, res) = track.download(&downloader).unwrap();
        fut.await.unwrap();
        let final_progress = res.rx.await.unwrap().borrow().clone();
        assert!(final_progress.downloaded == final_progress.total);
    }

    #[test]
    async fn test_download_album() {
        let (client, downloader) = make_client_and_downloader().await;
        let album = client
            .get_album("lz75qrx8pnjac")
            .await
            .map_err(|e| {
                println!("{e:?}");
                e
            })
            .unwrap();
        let (fut, res) = album.download(&downloader).unwrap();
        fut.await.unwrap();
        let final_progress = res.rx.await.unwrap().borrow().clone();
        assert!(final_progress.position == final_progress.total - 1);
    }
}
