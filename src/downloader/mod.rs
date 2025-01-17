use crate::{
    quality::{FileExtension, Quality},
    types::{
        extra::{ExtraFlag, WithExtra, WithoutExtra},
        Album, Array, Track,
    },
    ApiError,
};
use futures::{stream, StreamExt};
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::fs::OpenOptions;
pub mod tagging;
use tagging::{tag_track, TaggingError};

#[derive(Debug, Clone)]
pub struct Downloader {
    client: crate::Client,
    root: Box<Path>,
}

impl Downloader {
    /// Create a new `Downloader` which will use the given `Client` to download to the given
    /// `Path`.
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
    /// let root = Path::new("music");
    /// let downloader = Downloader::new(client, root);
    /// # })
    /// ```
    #[must_use]
    pub fn new(client: crate::Client, root: &Path) -> Self {
        Self {
            client,
            root: root.into(),
        }
    }

    /// Download and tag a track, returning the download locations of the album and track.
    ///
    /// # Example
    ///
    /// ```
    /// # use tokio_test;
    /// # tokio_test::block_on(async {
    /// # use qobuz::{auth::Credentials, Client, downloader::Downloader, quality::Quality};
    /// # use std::path::Path;
    /// # let credentials = Credentials::from_env().unwrap();
    /// # let client = Client::new(credentials).await.unwrap();
    /// # let root = Path::new("music");
    /// let downloader = Downloader::new(client.clone(), root);
    /// // Download "Let It Be", replacing the file if it already exists.
    /// let track = client
    ///     .get_track("129342731")
    ///     .await
    ///     .unwrap();
    /// downloader
    ///     .download_and_tag_track(&track, &track.album, Quality::Mp3, true)
    ///     .await
    ///     .unwrap();
    /// # })
    /// ```
    pub async fn download_and_tag_track<EF1: ExtraFlag, EF2: ExtraFlag>(
        &self,
        track: &Track<EF1>,
        album: &Album<EF2>,
        quality: Quality,
        force: bool,
    ) -> Result<(PathBuf, PathBuf), DownloadError>
    where
        <EF1 as ExtraFlag>::Extra<Album<WithoutExtra>>: Sync,
        <EF2 as ExtraFlag>::Extra<Array<Track<WithoutExtra>>>: Sync,
    {
        let album_path = self.get_standard_album_location(album, true)?;
        let track_path = self
            .download_track(track, &album_path, quality, force)
            .await?;
        let cover_raw = reqwest::get(album.image.large.clone())
            .await?
            .bytes()
            .await?;
        let cover = audiotags::Picture::new(&cover_raw, audiotags::MimeType::Jpeg);
        tag_track(track, &track_path, album, cover)?;
        Ok((album_path, track_path))
    }

    /// Download and tag an album, returning its download location.
    ///
    /// # Example
    ///
    ///
    /// ```
    /// # use tokio_test;
    /// # tokio_test::block_on(async {
    /// # use qobuz::{auth::Credentials, Client, downloader::Downloader, quality::Quality};
    /// # use std::path::Path;
    /// # let credentials = Credentials::from_env().unwrap();
    /// # let client = Client::new(credentials).await.unwrap();
    /// # let root = Path::new("music");
    /// # let downloader = Downloader::new(client.clone(), root);
    /// // Download "Abbey Road", replacing files if they already exist.
    /// let album = client
    ///     .get_album("trrcz9pvaaz6b")
    ///     .await
    ///     .unwrap();
    /// downloader
    ///     .download_and_tag_album(&album, Quality::Mp3, true)
    ///     .await
    ///     .unwrap();
    /// # })
    pub async fn download_and_tag_album(
        &self,
        album: &Album<WithExtra>,
        quality: Quality,
        force: bool,
    ) -> Result<(PathBuf, Vec<PathBuf>), DownloadError> {
        let album_path = self.get_standard_album_location(album, true)?;
        let cover_raw = reqwest::get(album.image.large.clone())
            .await?
            .bytes()
            .await?;
        let cover = audiotags::Picture::new(&cover_raw, audiotags::MimeType::Jpeg);
        let items = &album.tracks.items;

        let track_paths: Vec<PathBuf> = stream::iter(items)
            .then(|track| async {
                let track_path = self
                    .download_track(track, &album_path, quality.clone(), force)
                    .await?;
                tag_track(track, &track_path, album, cover.clone())?;
                Ok(track_path)
            })
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<_, DownloadError>>()?;

        Ok((album_path, track_paths))
    }

    async fn download_track<EF: ExtraFlag>(
        &self,
        track: &Track<EF>,
        album_path: &Path,
        quality: Quality,
        force: bool,
    ) -> Result<PathBuf, DownloadError>
    where
        <EF as ExtraFlag>::Extra<Album<WithoutExtra>>: Sync,
    {
        let track_path = self.get_standard_track_location(track, album_path, &quality);
        let mut out = match OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .create_new(!force) // (Shadows create and truncate)
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
            .stream_track(&track.id.to_string(), quality)
            .await?;
        while let Some(item) = bytes_stream.next().await {
            tokio::io::copy(&mut item?.as_ref(), &mut out).await?;
        }
        Ok(track_path)
    }

    // TODO: configurable path format
    pub fn get_standard_album_location<E: ExtraFlag>(
        &self,
        album: &Album<E>,
        ensure_exists: bool,
    ) -> Result<PathBuf, std::io::Error> {
        let mut path = self.root.to_path_buf();
        path.push(format!(
            "{} - {}",
            sanitize_filename(&album.artist.name),
            sanitize_filename(&album.title),
        ));
        if ensure_exists && !path.is_dir() {
            std::fs::create_dir_all(&path)?;
        }
        Ok(path)
    }

    #[must_use]
    pub fn get_standard_track_location<EF>(
        &self,
        track: &Track<EF>,
        album_path: &Path,
        quality: &Quality,
    ) -> PathBuf
    where
        EF: ExtraFlag,
    {
        let mut path = album_path.to_path_buf();
        path.push(sanitize_filename(&track.title));
        path.set_extension(FileExtension::from(quality).to_string());
        path
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
    const QUALITIES: [Quality; 4] = [
        Quality::Mp3,
        Quality::Cd,
        Quality::HiRes96,
        Quality::HiRes192,
    ];

    #[test]
    async fn test_download_and_tag_track() {
        let (client, downloader) = make_client_and_downloader().await;
        let track = client.get_track(HIRES192_TRACK).await.unwrap();
        for quality in QUALITIES {
            downloader
                .download_and_tag_track(&track, &track.album, quality.clone(), true)
                .await
                .unwrap();
        }
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
        downloader
            .download_and_tag_album(&album, Quality::Mp3, true)
            .await
            .unwrap();
    }
}
