use crate::{
    quality::{FileExtension, Quality},
    types::{
        extra::{ExtraFlag, WithExtra, WithoutExtra},
        Album, Array, Playlist, Track,
    },
    ApiError,
};
use futures::{stream, StreamExt};
use std::{
    ffi::OsStr,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};
use thiserror::Error;
use tokio::{fs::OpenOptions, io::AsyncWriteExt};
pub mod tagging;
use tagging::{tag_track, TaggingError};

#[derive(Debug, Clone)]
pub struct Downloader {
    client: crate::Client,
    root: Box<Path>,
    m3u_dir: Box<Path>,
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
        })
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
    /// let downloader = Downloader::new(
    ///    client.clone(),
    ///    Path::new("music"),
    ///    Path::new("music/playlists")
    /// ).unwrap();
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
    pub async fn download_and_tag_track<EF1, EF2>(
        &self,
        track: &Track<EF1>,
        album: &Album<EF2>,
        quality: Quality,
        force: bool,
    ) -> Result<(PathBuf, PathBuf), DownloadError>
    where
        EF1: ExtraFlag<Album<WithoutExtra>>,
        EF2: ExtraFlag<Array<Track<WithoutExtra>>>,
        EF1::Extra: Sync,
        EF2::Extra: Sync,
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
    /// # let downloader = Downloader::new(
    /// #    client.clone(),
    /// #    Path::new("music"),
    /// #    Path::new("music/playlists")
    /// # ).unwrap();
    /// // Download "One Last Time", replacing files if they already exist.
    /// let album = client
    ///     .get_album("lz75qrx8pnjac")
    ///     .await
    ///     .unwrap();
    /// downloader
    ///     .download_and_tag_album(&album, Quality::Mp3, true)
    ///     .await
    ///     .unwrap();
    /// # })
    /// ```
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

    /// Download and tag a playlist, creating an m3u file and returning download locations of the
    /// files.
    ///
    /// # Example
    /// ```rust,ignore
    /// # use tokio_test;
    /// # tokio_test::block_on(async {
    /// # use qobuz::{auth::Credentials, Client, downloader::Downloader, quality::Quality};
    /// # use std::path::Path;
    /// # let credentials = Credentials::from_env().unwrap();
    /// # let client = Client::new(credentials).await.unwrap();
    /// # let root = Path::new("music");
    /// # let downloader = Downloader::new(
    /// #    client.clone(),
    /// #    Path::new("music"),
    /// #    Path::new("music/playlists")
    /// # ).unwrap();
    /// // Download a playlist, replacing files if they already exist.
    /// let playlist = client
    ///     .get_playlist("2197152")
    ///     .await
    ///     .unwrap();
    /// downloader
    ///     .download_playlist_and_write_m3u(&playlist, Quality::Mp3, true)
    ///     .await
    ///     .unwrap();
    /// # })
    /// ```
    pub async fn download_playlist_and_write_m3u(
        &self,
        playlist: &Playlist<WithExtra>,
        quality: Quality,
        force: bool,
    ) -> Result<(PathBuf, Vec<PathBuf>), DownloadError> {
        let track_paths = self
            .download_tracks(&playlist.tracks.items, quality, force)
            .await?;
        let m3u_path = self.write_m3u(&playlist.name, &track_paths, force).await?;

        Ok((m3u_path, track_paths))
    }

    /// Download multiple tracks and return their paths.
    pub async fn download_tracks(
        &self,
        tracks: &[Track<WithExtra>],
        quality: Quality,
        force: bool,
    ) -> Result<Vec<PathBuf>, DownloadError> {
        let track_paths: Vec<PathBuf> = stream::iter(tracks)
            .then(|track| {
                let quality = quality.clone();
                async move {
                    let track_path = self
                        .download_and_tag_track(track, &track.album, quality, force)
                        .await?
                        .1;
                    Ok(track_path)
                }
            })
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<_, DownloadError>>()?;

        Ok(track_paths)
    }

    /// Write an M3U file for a playlist with a certain `name`, containing the already downloaded
    /// tracks `track_paths`, returning the new M3U file's path.
    pub async fn write_m3u(
        &self,
        name: &str,
        track_paths: &[PathBuf],
        force: bool,
    ) -> Result<PathBuf, std::io::Error> {
        let m3u_path = self.get_standard_m3u_location(name);
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .create_new(!force)
            .open(&m3u_path)
            .await?;
        let track_paths: Vec<&OsStr> = track_paths.iter().map(|p| p.as_os_str()).collect();
        let track_paths = track_paths.join(OsStr::from_bytes(b"\n"));
        file.write_all(track_paths.as_encoded_bytes()).await?;

        Ok(m3u_path)
    }

    async fn download_track<EF>(
        &self,
        track: &Track<EF>,
        album_path: &Path,
        quality: Quality,
        force: bool,
    ) -> Result<PathBuf, DownloadError>
    where
        EF: ExtraFlag<Album<WithoutExtra>>,
        EF::Extra: Sync,
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
    pub fn get_standard_album_location<EF>(
        &self,
        album: &Album<EF>,
        ensure_exists: bool,
    ) -> Result<PathBuf, std::io::Error>
    where
        EF: ExtraFlag<Array<Track<WithoutExtra>>>,
    {
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
        EF: ExtraFlag<Album<WithoutExtra>>,
    {
        let mut path = album_path.to_path_buf();
        path.push(sanitize_filename(&track.title));
        path.set_extension(FileExtension::from(quality).to_string());
        path
    }

    #[must_use]
    pub fn get_standard_m3u_location(&self, name: &str) -> PathBuf {
        let mut path = self.m3u_dir.to_path_buf();
        path.push(sanitize_filename(name));
        path.set_extension("m3u");
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
