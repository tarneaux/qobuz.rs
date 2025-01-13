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
    #[must_use]
    pub fn new(client: crate::Client, root: &Path) -> Self {
        Self {
            client,
            root: root.into(),
        }
    }

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
            .create_new(true)
            .open(&track_loc)
            .await
        {
            Ok(v) => v,
            Err(e) => {
                return match e.kind() {
                    std::io::ErrorKind::AlreadyExists if !force => Ok(track_loc),
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
