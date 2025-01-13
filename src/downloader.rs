use crate::{extra::Extra, tag_track, Album, ApiError, FileType, Quality, TaggingError, Track};
use futures::StreamExt;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::fs::OpenOptions;

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
    /// use qobuz::{QobuzCredentials, Client, Downloader};
    /// use std::path::Path;
    /// let credentials = QobuzCredentials::from_env().unwrap();
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

    /// Download and tag a track.
    ///
    /// # Example
    ///
    /// ```
    /// # use tokio_test;
    /// # tokio_test::block_on(async {
    /// # use qobuz::{QobuzCredentials, Client, Downloader};
    /// # use std::path::Path;
    /// # let credentials = QobuzCredentials::from_env().unwrap();
    /// # let client = Client::new(credentials).await.unwrap();
    /// # let root = Path::new("music");
    /// use qobuz::Quality;
    /// let downloader = Downloader::new(client.clone(), root);
    /// // Download "Let It Be", replacing the file if it already exists.
    /// let track = client
    ///     .get_track("129342731")
    ///     .await
    ///     .unwrap();
    /// downloader.download_and_tag_track(&track, &track.extra.album, Quality::Mp3, true);
    /// # })
    /// ```
    pub async fn download_and_tag_track<E1, E2>(
        &self,
        track: &Track<E1>,
        album: &Album<E2>,
        quality: Quality,
        force: bool,
    ) -> Result<PathBuf, DownloadError>
    where
        Track<E1>: Extra + Send,
        Album<E2>: Extra + Send,
        E1: Sync,
        E2: Sync,
    {
        let track_loc = self.download_track(track, album, quality, force).await?;
        let cover_raw = reqwest::get(album.image.large.clone())
            .await?
            .bytes()
            .await?;
        let cover = audiotags::Picture::new(&cover_raw, audiotags::MimeType::Jpeg);
        tag_track(track, &track_loc, album, cover)?;
        Ok(track_loc)
    }

    async fn download_track<E1, E2>(
        &self,
        track: &Track<E1>,
        album: &Album<E2>,
        quality: Quality,
        force: bool,
    ) -> Result<PathBuf, DownloadError>
    where
        Track<E1>: Extra + Send,
        Album<E2>: Extra + Send,
        E1: Sync,
        E2: Sync,
    {
        self.ensure_album_dir_exists(album)?;
        let track_loc = get_standard_track_location(&self.root, track, album, &quality);
        let mut out = match OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .create_new(!force) // (Shadows create and truncate)
            .open(&track_loc)
            .await
        {
            Ok(v) => v,
            Err(e) => {
                return match e.kind() {
                    std::io::ErrorKind::AlreadyExists => Ok(track_loc),
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
        Ok(track_loc)
    }

    fn ensure_album_dir_exists<E>(&self, album: &Album<E>) -> Result<(), std::io::Error>
    where
        Album<E>: Extra,
    {
        let album_loc = get_standard_album_location(&self.root, album);
        if !album_loc.is_dir() {
            std::fs::create_dir_all(&album_loc)?;
        }
        Ok(())
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

// TODO: configurable path format
#[must_use]
pub fn get_standard_album_location<E>(root: &Path, album: &Album<E>) -> PathBuf
where
    Album<E>: Extra,
{
    let mut path = root.to_path_buf();
    path.push(format!(
        "{} - {}",
        sanitize_filename(&album.artist.name),
        sanitize_filename(&album.title),
    ));
    path
}

#[must_use]
pub fn get_standard_track_location<E1, E2>(
    root: &Path,
    track: &Track<E1>,
    album: &Album<E2>,
    quality: &Quality,
) -> PathBuf
where
    Track<E1>: Extra,
    Album<E2>: Extra,
{
    let mut path = get_standard_album_location(root, album);
    path.push(sanitize_filename(&track.title));
    path.set_extension(FileType::from(quality).to_string());
    path
}

#[must_use]
pub fn sanitize_filename(filename: &str) -> String {
    let filename = filename.trim().replace('/', "-");
    filename.trim_start_matches('.').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::make_client_and_downloader;
    use tokio::test;

    const HIRES192_TRACK: &str = "18893849"; // Creedence Clearwater Revival - Lodi

    #[test]
    async fn test_download_and_tag_track() {
        let (client, downloader) = make_client_and_downloader().await;
        let track = client
            .get_track(HIRES192_TRACK)
            .await
            .unwrap_or_else(|_| panic!("Couldn't get track {HIRES192_TRACK}"));
        for quality in [
            Quality::Mp3,
            Quality::Cd,
            Quality::HiRes96,
            Quality::HiRes192,
        ] {
            downloader
                .download_and_tag_track(&track, &track.extra.album, quality.clone(), true)
                .await
                .unwrap_or_else(|_| {
                    panic!("Couldn't download hires192 track in quality {quality:?}")
                });
        }
    }
}
