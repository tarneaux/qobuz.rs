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
    ffi::OsStr,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};
use thiserror::Error;
use tokio::{fs::OpenOptions, io::AsyncWriteExt};
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
    pub async fn write_m3u(
        &self,
        playlist: &Playlist<WithExtra>,
        track_paths: &[PathBuf],
    ) -> Result<PathBuf, DownloadError> {
        let m3u_path = self.get_m3u_path(playlist)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&m3u_path)
            .await?;
        let track_paths: Vec<&OsStr> = track_paths
            .iter()
            .map(|p| {
                p.strip_prefix(&self.root)
                    .expect("Path should be relative to music directory")
                    .as_os_str()
            })
            .collect();
        let track_paths = track_paths.join(OsStr::from_bytes(b"\n"));
        file.write_all(track_paths.as_encoded_bytes()).await?;

        Ok(m3u_path)
    }

    async fn download_track<EF>(
        &self,
        track: &Track<EF>,
        album_path: &Path,
    ) -> Result<PathBuf, DownloadError>
    where
        EF: ExtraFlag<Album<WithoutExtra>>,
        EF::Extra: Sync,
    {
        let track_path = self.get_track_path(track, album_path)?;
        let mut out = match OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .create_new(!self.overwrite) // (Shadows create and truncate)
            .open(&track_path)
            .await
        {
            Ok(v) => v,
            Err(e) => {
                return match e.kind() {
                    std::io::ErrorKind::AlreadyExists => Ok(track_path),
                    _ => Err(DownloadError::IoError(e)),
                }
            }
        };
        let mut bytes_stream = self
            .client
            .stream_track(&track.id.to_string(), self.quality.clone())
            .await?;
        while let Some(item) = bytes_stream.next().await {
            tokio::io::copy(&mut item?.as_ref(), &mut out).await?;
        }
        Ok(track_path)
    }

    pub fn get_album_path<EF>(
        &self,
        album: &Album<EF>,
        ensure_exists: bool,
    ) -> Result<PathBuf, DownloadError>
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
    type DRT;
    fn download_and_tag(
        &self,
        downloader: &Downloader,
    ) -> impl Future<Output = Result<Self::DRT, DownloadError>>;
}

impl Download for Track<WithExtra> {
    type DRT = (PathBuf, PathBuf);
    /// Download and tag a track, returning the download locations of the album and track.
    async fn download_and_tag(&self, downloader: &Downloader) -> Result<Self::DRT, DownloadError> {
        let album_path = downloader.get_album_path(&self.album, true)?;
        let track_path = downloader.download_track(self, &album_path).await?;
        let cover_raw = reqwest::get(self.album.image.large.clone())
            .await?
            .bytes()
            .await?;
        let cover = audiotags::Picture::new(&cover_raw, audiotags::MimeType::Jpeg);
        tag_track(self, &track_path, &self.album, cover)?;
        Ok((album_path, track_path))
    }
}

impl Download for Album<WithExtra> {
    type DRT = (PathBuf, Vec<PathBuf>);
    /// Download and tag an album, returning its root directory and it's tracks paths.
    async fn download_and_tag(&self, downloader: &Downloader) -> Result<Self::DRT, DownloadError> {
        let album_path = downloader.get_album_path(self, true)?;
        let cover_raw = reqwest::get(self.image.large.clone())
            .await?
            .bytes()
            .await?;
        let cover = audiotags::Picture::new(&cover_raw, audiotags::MimeType::Jpeg);
        let items = &self.tracks.items;

        let track_paths: Vec<PathBuf> = futures::stream::iter(items)
            .then(|track| async {
                let track_path = downloader.download_track(track, &album_path).await?;
                tag_track(track, &track_path, self, cover.clone())?;
                Ok(track_path)
            })
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<_, DownloadError>>()?;

        Ok((album_path, track_paths))
    }
}

impl Download for Playlist<WithExtra> {
    type DRT = (PathBuf, Vec<PathBuf>);
    /// Download and tag a playlist, creating an m3u file and returning download locations of the
    /// files.
    async fn download_and_tag(&self, downloader: &Downloader) -> Result<Self::DRT, DownloadError> {
        let track_paths: Vec<PathBuf> = futures::stream::iter(&self.tracks.items)
            .then(|track| async move {
                let track_path = track.download_and_tag(downloader).await?.1;
                Ok(track_path)
            })
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<_, DownloadError>>()?;
        let m3u_path = downloader.write_m3u(self, &track_paths).await?;

        Ok((m3u_path, track_paths))
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
    async fn test_download_and_tag_track() {
        let (client, downloader) = make_client_and_downloader().await;
        let track = client.get_track(HIRES192_TRACK).await.unwrap();
        track.download_and_tag(&downloader).await.unwrap();
    }

    #[test]
    async fn test_download_and_tag_album() {
        let (client, downloader) = make_client_and_downloader().await;
        let album = client
            .get_album("lz75qrx8pnjac")
            .await
            .map_err(|e| {
                println!("{e:?}");
                e
            })
            .unwrap();
        album.download_and_tag(&downloader).await.unwrap();
    }
}
