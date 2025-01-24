pub mod auth;
pub mod downloader;
pub mod quality;
pub mod types;

#[cfg(test)]
mod test_utils;

use crate::{
    auth::{get_user_auth_token, Credentials, LoginError},
    quality::Quality,
    types::{
        extra::{RootEntity, WithExtra, WithoutExtra},
        traits::Favoritable,
        Album, Array, Artist, Playlist, QobuzType, Track,
    },
};
use bytes::Bytes;
use futures::Stream;
use serde::de::DeserializeOwned;
use serde_json::Value;
use thiserror::Error;

const API_URL: &str = "https://www.qobuz.com/api.json/0.2/";
const API_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:83.0) Gecko/20100101 Firefox/83.0";

#[derive(Debug, Clone)]
pub struct Client {
    pub reqwest_client: reqwest::Client,
    secret: String,
}

impl Client {
    /// Create a new `Client`, logging in with the given credentials.
    ///
    /// # Example
    ///
    /// ```
    /// # use tokio_test;
    /// # tokio_test::block_on(async {
    /// use qobuz::{auth::Credentials, Client};
    /// let credentials = Credentials::from_env().unwrap();
    /// let client = Client::new(credentials).await.unwrap();
    /// # })
    /// ```
    pub async fn new(credentials: Credentials) -> Result<Self, LoginError> {
        let uat = get_user_auth_token(&credentials).await?;
        let reqwest_client = make_http_client(&credentials.app_id, Some(&uat));

        Ok(Self {
            reqwest_client,
            secret: credentials.secret,
        })
    }

    /// Get the download URL of a track.
    ///
    /// # Example
    ///
    /// ```
    /// # tokio_test::block_on(async {
    /// # use qobuz::{auth::Credentials, Client};
    /// # let credentials = Credentials::from_env().unwrap();
    /// # let client = Client::new(credentials).await.unwrap();
    /// use qobuz::quality::Quality;
    /// // Get download URL of "Let it Be" (the track)
    /// let track = client
    ///     .get_track_file_url("129342731", Quality::HiRes96)
    ///     .await
    ///     .unwrap();
    /// # })
    /// ```
    pub async fn get_track_file_url(
        &self,
        track_id: &str, // TODO: u64?
        quality: Quality,
    ) -> Result<url::Url, ApiError> {
        let timestamp_now = chrono::Utc::now().timestamp().to_string();

        let quality_id: u8 = quality.into();

        let r_sig_hash = format!(
            "{:x}",
            md5::compute(format!(
                "trackgetFileUrlformat_id{}intentstreamtrack_id{}{}{}",
                quality_id, track_id, timestamp_now, self.secret
            ))
        );

        let params = [
            ("request_ts", timestamp_now.as_str()),
            ("request_sig", &r_sig_hash),
            ("track_id", track_id),
            ("format_id", &quality_id.to_string()),
            ("intent", "stream"),
        ];
        let res: Value = self.do_request("track/getFileUrl", &params).await?;
        if res.get("sample") == Some(&Value::Bool(true)) {
            return Err(ApiError::IsSample);
        }
        let url: serde_json::Value = res
            .get("url")
            .ok_or(ApiError::MissingKey("url".to_string()))?
            .clone();
        Ok(serde_json::from_value(url)?)
    }

    /// Get the user's favorites of type `T`.
    ///
    /// # Example
    ///
    /// ```
    /// # tokio_test::block_on(async {
    /// # use qobuz::{auth::Credentials, Client};
    /// # let credentials = Credentials::from_env().unwrap();
    /// # let client = Client::new(credentials).await.unwrap();
    /// use qobuz::types::{Track, extra::WithExtra};
    /// // Get the user's favorite tracks
    /// let favorites = client.get_user_favorites::<Track<WithExtra>>().await.unwrap();
    /// # })
    /// ```
    pub async fn get_user_favorites<T: QobuzType + DeserializeOwned + Favoritable>(
        &self,
    ) -> Result<Vec<T>, ApiError> {
        let fav_type = T::name_plural();
        let params = [
            ("type", fav_type),
            ("limit", "500"),
            ("offset", "0"), // TODO: walk
        ];
        let res: Value = self
            .do_request("favorite/getUserFavorites", &params)
            .await?;
        let array: Value = res
            .get(fav_type)
            .ok_or(ApiError::MissingKey(fav_type.to_string()))?
            .clone();
        let array: Array<T> = serde_json::from_value(array)?;
        Ok(array.items)
    }

    /// Get the user's playlists.
    ///
    /// # Example
    ///
    /// ```
    /// # tokio_test::block_on(async {
    /// # use qobuz::{auth::Credentials, Client};
    /// # let credentials = Credentials::from_env().unwrap();
    /// # let client = Client::new(credentials).await.unwrap();
    /// // Get the user's playlists
    /// let playlists = client.get_user_playlists().await.unwrap();
    /// # })
    /// ```
    pub async fn get_user_playlists(&self) -> Result<Vec<Playlist<WithoutExtra>>, ApiError> {
        let params = [
            ("limit", "500"),
            ("offset", "0"), // TODO: walk
        ];
        let res: Value = self
            .do_request("playlist/getUserPlaylists", &params)
            .await?;
        let array: Value = res
            .get("playlists")
            .ok_or(ApiError::MissingKey("playlists".to_string()))?
            .clone();
        let array: Array<Playlist<WithoutExtra>> = serde_json::from_value(array)?;
        Ok(array.items)
    }

    /// Get information on an item.
    ///
    /// # Example
    ///
    /// ```
    /// # tokio_test::block_on(async {
    /// # use qobuz::{auth::Credentials, Client};
    /// # let credentials = Credentials::from_env().unwrap();
    /// # let client = Client::new(credentials).await.unwrap();
    /// use qobuz::{types::Track, types::extra::WithExtra};
    /// // Get information on "Let It Be" (the track)
    /// let track = client
    ///     .get_item::<Track<WithExtra>>("129342731")
    ///     .await
    ///     .unwrap();
    /// # })
    /// ```
    pub async fn get_item<T>(&self, id: &str) -> Result<T, ApiError>
    where
        T: QobuzType + RootEntity + DeserializeOwned,
    {
        Ok(self
            .do_request(
                &format!("{}/get", T::name_singular()),
                &[
                    (format!("{}_id", T::name_singular()).as_str(), id),
                    ("extra", T::extra_arg()),
                    ("limit", "500"), // TODO: walk
                    ("offset", "0"),
                ],
            )
            .await?)
    }

    /// Get information on a track.
    ///
    /// # Example
    ///
    /// ```
    /// # tokio_test::block_on(async {
    /// # use qobuz::{auth::Credentials, Client};
    /// # let credentials = Credentials::from_env().unwrap();
    /// # let client = Client::new(credentials).await.unwrap();
    /// // Get information on "Let It Be" (the track)
    /// let track = client
    ///     .get_track("129342731")
    ///     .await
    ///     .unwrap();
    /// # })
    /// ```
    pub async fn get_track(&self, track_id: &str) -> Result<Track<WithExtra>, ApiError> {
        self.get_item(track_id).await
    }

    /// Get information on a playlist.
    ///
    /// # Example
    ///
    /// ```
    /// # tokio_test::block_on(async {
    /// # use qobuz::{auth::Credentials, Client};
    /// # let credentials = Credentials::from_env().unwrap();
    /// # let client = Client::new(credentials).await.unwrap();
    /// // Get information on an official Beatles playlist
    /// let playlist = client
    ///     .get_playlist("1141084")
    ///     .await
    ///     .unwrap();
    /// # })
    /// ```
    pub async fn get_playlist(&self, playlist_id: &str) -> Result<Playlist<WithExtra>, ApiError> {
        self.get_item(playlist_id).await
    }

    /// Get information on an album.
    ///
    /// # Example
    ///
    /// ```
    /// # tokio_test::block_on(async {
    /// # use qobuz::{auth::Credentials, Client};
    /// # let credentials = Credentials::from_env().unwrap();
    /// # let client = Client::new(credentials).await.unwrap();
    /// // Get information on "Abbey Road"
    /// let album = client
    ///     .get_album("trrcz9pvaaz6b")
    ///     .await
    ///     .unwrap();
    /// # })
    /// ```
    pub async fn get_album(&self, album_id: &str) -> Result<Album<WithExtra>, ApiError> {
        self.get_item(album_id).await
    }

    /// Get information on an artist.
    ///
    /// # Example
    ///
    /// ```
    /// # tokio_test::block_on(async {
    /// # use qobuz::{auth::Credentials, Client};
    /// # let credentials = Credentials::from_env().unwrap();
    /// # let client = Client::new(credentials).await.unwrap();
    /// // Get information on the Beatles
    /// let artist = client
    ///     .get_artist("26390")
    ///     .await
    ///     .unwrap();
    /// # })
    /// ```
    pub async fn get_artist(&self, artist_id: &str) -> Result<Artist<WithExtra>, ApiError> {
        self.get_item(artist_id).await
    }

    /// Stream a track.
    ///
    /// # Example
    ///
    /// ```
    /// # tokio_test::block_on(async {
    /// use tokio::fs::File;
    /// use futures::StreamExt;
    /// # use qobuz::{auth::Credentials, Client, quality::Quality};
    /// # let credentials = Credentials::from_env().unwrap();
    /// # let client = Client::new(credentials).await.unwrap();
    /// // Download the "Let It Be" track to test.mp3
    /// let mut bytes_stream = client
    ///     .stream_track("129342731", Quality::HiRes96)
    ///     .await
    ///     .unwrap();
    /// let mut out = File::create("let_it_be.mp3")
    ///     .await
    ///     .expect("failed to create file");
    /// while let Some(item) = bytes_stream.next().await {
    ///     tokio::io::copy(&mut item.unwrap().as_ref(), &mut out)
    ///         .await
    ///         .unwrap();
    /// }
    /// # })
    /// ```
    pub async fn stream_track(
        &self,
        track_id: &str,
        quality: Quality,
    ) -> Result<impl Stream<Item = reqwest::Result<Bytes>>, ApiError> {
        let url = self.get_track_file_url(track_id, quality).await?;
        Ok(self.reqwest_client.get(url).send().await?.bytes_stream())
    }

    async fn do_request<T: DeserializeOwned>(
        &self,
        path: &str,
        params: &[(&str, &str)],
    ) -> Result<T, reqwest::Error> {
        do_request(&self.reqwest_client, path, params).await
    }
}

async fn do_request<T: DeserializeOwned>(
    client: &reqwest::Client,
    path: &str,
    params: &[(&str, &str)],
) -> Result<T, reqwest::Error> {
    let url = format!("{API_URL}{path}");
    let res = client
        .get(&url)
        .query(params)
        .send()
        .await?
        .error_for_status();

    #[cfg(test)]
    {
        #![allow(clippy::unwrap_used)]
        if res.as_ref().is_err_and(reqwest::Error::is_status) {
            println!(
                "Got status error while querying {url}. Querying again to hopefully replicate the error..."
            );
            let res = client.get(url).query(params).send().await?;
            if res.status().is_success() {
                println!("Replicating the error failed: the status is a success");
            }
            println!("Status code: {}", res.status());
            println!("Text: {}", res.text().await.unwrap());
        }
    }

    res?.json().await
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("downloadable file is a sample")]
    IsSample,
    #[error("couldn't get key `{0}`")]
    MissingKey(String),
    #[error("serde_json error `{0}`")]
    SerdeJsonError(#[from] serde_json::Error),
    #[error("reqwest error `{0}`")]
    ReqwestError(#[from] reqwest::Error),
}

fn make_http_client(app_id: &str, uat: Option<&str>) -> reqwest::Client {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("X-App-Id", app_id.parse().expect("Failed to parse app id"));
    headers.insert(
        reqwest::header::CONTENT_TYPE,
        "application/json;charset=UTF-8"
            .parse()
            .expect("Coudln't parse static content type"),
    );
    if let Some(uat) = uat {
        headers.insert(
            "X-User-Auth-Token",
            uat.parse().expect("Coudln't parse user auth token"),
        );
    }
    reqwest::ClientBuilder::new()
        .user_agent(API_USER_AGENT)
        .default_headers(headers)
        .build()
        .expect("Couldn't build reqwest::Client")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::test_utils::make_client;
    use tokio::test;

    #[test]
    async fn test_get_user_favorites() {
        let client = make_client().await;
        client
            .get_user_favorites::<Album<WithoutExtra>>()
            .await
            .unwrap();
        client
            .get_user_favorites::<Track<WithExtra>>()
            .await
            .unwrap();
        client
            .get_user_favorites::<Artist<WithoutExtra>>()
            .await
            .unwrap();
    }

    #[test]
    async fn test_get_user_playlists() {
        let client = make_client().await;
        client.get_user_playlists().await.unwrap();
    }

    #[test]
    async fn test_get_track_file_url() {
        let track_id = "64868955";
        make_client()
            .await
            .get_track_file_url(track_id, Quality::HiRes96)
            .await
            .unwrap();
    }

    #[test]
    async fn test_get_track() {
        let client = make_client().await;
        let track_id = "64868955";
        client.get_track(track_id).await.unwrap();
        client.get_track("no").await.unwrap_err();
    }

    #[test]
    async fn test_get_album() {
        let client = make_client().await;
        let album_id = "trrcz9pvaaz6b";
        client.get_album(album_id).await.unwrap();
        client.get_album("no").await.unwrap_err();
    }

    #[test]
    async fn test_get_artist() {
        let client = make_client().await;
        let artist_id = "26390";
        client.get_artist(artist_id).await.unwrap();
        client.get_artist("no").await.unwrap_err();
    }

    #[test]
    async fn test_get_playlist() {
        let client = make_client().await;
        let playlist_id = "1141084"; // Official Qobuz playlist
        client.get_playlist(playlist_id).await.unwrap();
        client.get_playlist("no").await.unwrap_err();
        // TODO: First user playlist
    }

    #[test]
    async fn test_stream_track() {
        use futures::StreamExt;
        let mut stream = make_client()
            .await
            .stream_track("64868955", Quality::HiRes96)
            .await
            .unwrap();
        assert!(stream.next().await.is_some());
    }
}
