use bytes::Bytes;
use futures::Stream;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::env::VarError;
use thiserror::Error;

use crate::{extra, quality::Quality, Album, Array, Artist, Playlist, QobuzType, Track};

const API_URL: &str = "https://www.qobuz.com/api.json/0.2/";
const API_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:83.0) Gecko/20100101 Firefox/83.0";

#[derive(Debug, Clone)]
pub struct Client {
    pub reqwest_client: reqwest::Client,
    secret: String,
}

impl Client {
    /// Create a new client, logging in with the given credentials.
    ///
    /// # Example
    ///
    /// ```
    /// # use tokio_test;
    /// # tokio_test::block_on(async {
    /// use qobuz::{QobuzCredentials, Client};
    /// let credentials = QobuzCredentials::from_env().unwrap();
    /// let client = Client::new(credentials).await.unwrap();
    /// # })
    /// ```
    pub async fn new(credentials: QobuzCredentials) -> Result<Self, LoginError> {
        let token = get_auth_token(&credentials).await?;
        let reqwest_client = make_http_client(&credentials.app_id, Some(&token));

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
    /// # use qobuz::{QobuzCredentials, Client};
    /// # let credentials = QobuzCredentials::from_env().unwrap();
    /// # let client = Client::new(credentials).await.unwrap();
    /// use qobuz::Quality;
    /// // Get download URL of "Let it Be" (the track)
    /// let mut artist = client
    ///     .get_track_file_url("129342731", Quality::HiRes96)
    ///     .await
    ///     .unwrap();
    /// # })
    /// ```
    pub async fn get_track_file_url(
        &self,
        track_id: &str, // TODO: u64?
        quality: Quality,
    ) -> Result<DownloadUrl, ApiError> {
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
        let res: DownloadUrl = serde_json::from_value(res)?;
        Ok(res)
    }

    /// Get the user's favorites of type `T`.
    ///
    /// # Example
    ///
    /// ```
    /// # tokio_test::block_on(async {
    /// # use qobuz::{QobuzCredentials, Client};
    /// # let credentials = QobuzCredentials::from_env().unwrap();
    /// # let client = Client::new(credentials).await.unwrap();
    /// use qobuz::Track;
    /// // Get the user's favorite tracks
    /// client.get_user_favorites::<Track<()>>().await.unwrap();
    /// # })
    /// ```
    pub async fn get_user_favorites<T: QobuzType<Extra = ()>>(&self) -> Result<Vec<T>, ApiError> {
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
    /// # use qobuz::{QobuzCredentials, Client};
    /// # let credentials = QobuzCredentials::from_env().unwrap();
    /// # let client = Client::new(credentials).await.unwrap();
    /// // Get the user's favorite tracks
    /// client.get_user_playlists().await.unwrap();
    /// # })
    /// ```
    pub async fn get_user_playlists(&self) -> Result<Vec<Playlist<()>>, ApiError> {
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
        let array: Array<Playlist<()>> = serde_json::from_value(array)?;
        Ok(array.items)
    }

    /// Get information on an item.
    ///
    /// # Example
    ///
    /// ```
    /// # tokio_test::block_on(async {
    /// # use qobuz::{QobuzCredentials, Client};
    /// # let credentials = QobuzCredentials::from_env().unwrap();
    /// # let client = Client::new(credentials).await.unwrap();
    /// use qobuz::{Track, extra::AlbumAndComposer};
    /// // Get information on "Let It Be" (the track)
    /// let track = client
    ///     .get_item::<Track<()>>("129342731")
    ///     .await
    ///     .unwrap();
    /// # })
    /// ```
    pub async fn get_item<T>(&self, id: &str) -> Result<T, ApiError>
    where
        T: QobuzType,
    {
        Ok(self
            .do_request(
                &format!("{}/get", T::name_singular()),
                &[
                    (format!("{}_id", T::name_singular()).as_str(), id),
                    ("extra", T::extra_arg().unwrap_or("")),
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
    /// # use qobuz::{QobuzCredentials, Client};
    /// # let credentials = QobuzCredentials::from_env().unwrap();
    /// # let client = Client::new(credentials).await.unwrap();
    /// // Get information on "Let It Be" (the track)
    /// let track = client
    ///     .get_track("129342731")
    ///     .await
    ///     .unwrap();
    /// # })
    /// ```
    pub async fn get_track(
        &self,
        track_id: &str,
    ) -> Result<Track<extra::AlbumAndComposer>, ApiError> {
        self.get_item(track_id).await
    }

    /// Get information on a playlist.
    ///
    /// # Example
    ///
    /// ```
    /// # tokio_test::block_on(async {
    /// # use qobuz::{QobuzCredentials, Client};
    /// # let credentials = QobuzCredentials::from_env().unwrap();
    /// # let client = Client::new(credentials).await.unwrap();
    /// // Get information on an official Beatles playlist
    /// let playlist = client
    ///     .get_playlist("1141084")
    ///     .await
    ///     .unwrap();
    /// # })
    /// ```
    pub async fn get_playlist(
        &self,
        playlist_id: &str,
    ) -> Result<Playlist<extra::Tracks>, ApiError> {
        self.get_item(playlist_id).await
    }

    /// Get information on an album.
    ///
    /// # Example
    ///
    /// ```
    /// # tokio_test::block_on(async {
    /// # use qobuz::{QobuzCredentials, Client};
    /// # let credentials = QobuzCredentials::from_env().unwrap();
    /// # let client = Client::new(credentials).await.unwrap();
    /// // Get information on "Abbey Road"
    /// let album = client
    ///     .get_album("trrcz9pvaaz6b")
    ///     .await
    ///     .unwrap();
    /// # })
    /// ```
    pub async fn get_album(&self, album_id: &str) -> Result<Album<extra::Tracks>, ApiError> {
        self.get_item(album_id).await
    }

    /// Get information on an artist.
    ///
    /// # Example
    ///
    /// ```
    /// # tokio_test::block_on(async {
    /// # use qobuz::{QobuzCredentials, Client};
    /// # let credentials = QobuzCredentials::from_env().unwrap();
    /// # let client = Client::new(credentials).await.unwrap();
    /// // Get information on the Beatles
    /// let artist = client
    ///     .get_artist("26390")
    ///     .await
    ///     .unwrap();
    /// # })
    /// ```
    pub async fn get_artist(
        &self,
        artist_id: &str,
    ) -> Result<Artist<extra::AlbumsAndTracks>, ApiError> {
        self.get_item(artist_id).await
    }

    /// Stream a track.
    ///
    /// # Example
    ///
    /// ```
    /// # tokio_test::block_on(async {
    /// use qobuz::Quality;
    /// use tokio::fs::File;
    /// use futures::StreamExt;
    /// # use qobuz::{QobuzCredentials, Client};
    /// # let credentials = QobuzCredentials::from_env().unwrap();
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
        Ok(self
            .reqwest_client
            .get(url.inner)
            .send()
            .await?
            .bytes_stream())
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
    client
        .get(format!("{API_URL}{path}"))
        .query(params)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
}

async fn get_auth_token(credentials: &QobuzCredentials) -> Result<String, LoginError> {
    let client = make_http_client(&credentials.app_id, None);
    let params = [
        ("email", credentials.email.as_str()),
        ("password", credentials.password.as_str()),
        ("app_id", credentials.app_id.as_str()),
    ];
    let resp: Value = do_request(&client, "user/login", &params)
        .await
        .map_err(|e| match e.status() {
            Some(reqwest::StatusCode::UNAUTHORIZED) => LoginError::InvalidCredentials,
            Some(reqwest::StatusCode::BAD_REQUEST) => LoginError::InvalidAppId,
            _ => LoginError::ReqwestError(e),
        })?;
    // verify json["user"]["credential"]["parameters"] exists.
    // If not, we are authenticating into a free account which can't download tracks.
    resp.get("user")
        .ok_or(LoginError::FreeAccount)?
        .get("credential")
        .ok_or(LoginError::FreeAccount)?
        .get("parameters")
        .ok_or(LoginError::FreeAccount)?;
    match resp.get("user_auth_token") {
        Some(Value::String(token)) => Ok(token.to_string()),
        None | Some(_) => Err(LoginError::NoUserAuthToken),
    }
}

#[derive(Debug, Error)]
pub enum LoginError {
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("invialid app id")]
    InvalidAppId,
    #[error("reqwest error `{0}`")]
    ReqwestError(#[from] reqwest::Error),
    #[error("no user auth token")]
    NoUserAuthToken,
    #[error("tried to authenticate into a free account which can't download tracks")]
    FreeAccount,
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("downloadable file is a sample")]
    IsSample,
    #[error("couldn't get key `{0}`")]
    MissingKey(String),
    #[error("serde error `{0}`")]
    SerdeJson(#[from] serde_json::Error),
    #[error("reqwest error `{0}`")]
    Reqwest(#[from] reqwest::Error),
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DownloadUrl {
    #[serde(rename = "url")]
    pub inner: String,
    pub format_id: Quality,
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
    if let Some(token) = uat {
        headers.insert(
            "X-User-Auth-Token",
            token.parse().expect("Coudln't parse auth token"),
        );
    }
    reqwest::ClientBuilder::new()
        .user_agent(API_USER_AGENT)
        .default_headers(headers)
        .build()
        .expect("Couldn't build reqwest::Client")
}

pub async fn test_secret(app_id: &str, secret: String) -> Result<bool, ApiError> {
    if secret.is_empty() {
        return Ok(false);
    }
    let client = Client {
        reqwest_client: make_http_client(app_id, None),
        secret,
    };
    match client
        .get_track_file_url("64868958", Quality::HiRes192)
        .await
    {
        Err(ApiError::IsSample) => Ok(true),
        Err(ApiError::Reqwest(e)) => {
            if e.is_status() {
                Ok(false)
            } else {
                Err(ApiError::Reqwest(e))
            }
        }
        Err(e) => Err(e),
        // Since the X-User-Auth-Token header isn't set, we can't get a non-sample URL.
        Ok(_) => unreachable!(),
    }
}

/// Credentials for Qobuz.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QobuzCredentials {
    pub email: String,
    pub password: String,
    pub app_id: String,
    pub secret: String,
}

impl QobuzCredentials {
    /// Get the credentials from environment variables (`QOBUZ_*`).
    ///
    /// # Errors
    ///
    /// If an environment variable is missing.
    pub fn from_env() -> Result<Self, VarError> {
        Ok(Self {
            email: env::var("QOBUZ_EMAIL")?,
            password: env::var("QOBUZ_PASSWORD")?,
            app_id: env::var("QOBUZ_APP_ID")?,
            secret: env::var("QOBUZ_SECRET")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::make_client;
    use tokio::test;

    #[test]
    async fn test_get_user_favorites() {
        let client = make_client().await;
        client
            .get_user_favorites::<Album<()>>()
            .await
            .expect("Couldn't get user favorites of type Album");
        client
            .get_user_favorites::<Track<()>>()
            .await
            .expect("Couldn't get user favorites of type Track");
        client
            .get_user_favorites::<Artist<()>>()
            .await
            .expect("Couldn't get user favorites of type Artist");
    }

    #[test]
    async fn test_get_user_playlists() {
        let client = make_client().await;
        client
            .get_user_playlists()
            .await
            .expect("Couldn't get user playlists");
    }

    #[test]
    async fn test_get_track_file_url() {
        let track_id = "64868955";
        make_client()
            .await
            .get_track_file_url(track_id, Quality::HiRes96)
            .await
            .unwrap_or_else(|_| panic!("Couldn't get track file url for track {track_id}"));
    }

    #[test]
    async fn test_get_track() {
        let client = make_client().await;
        let track_id = "64868955";
        client
            .get_track(track_id)
            .await
            .unwrap_or_else(|_| panic!("Couldn't get track file url for track {track_id}"));
        client
            .get_track("no")
            .await
            .expect_err("There should be no track with id 'no'");
    }

    #[test]
    async fn test_get_album() {
        let client = make_client().await;
        let album_id = "trrcz9pvaaz6b";
        client
            .get_album(album_id)
            .await
            .unwrap_or_else(|_| panic!("Couldn't get album {album_id}"));
        client
            .get_album("no")
            .await
            .expect_err("There should be no album with id 'no'");
    }

    #[test]
    async fn test_get_artist() {
        let client = make_client().await;
        let artist_id = "26390";
        client
            .get_artist(artist_id)
            .await
            .unwrap_or_else(|_| panic!("Couldn't get artist {artist_id}"));
        client
            .get_artist("no")
            .await
            .expect_err("There should be no artist with id 'no'");
    }

    #[test]
    async fn test_get_playlist() {
        let client = make_client().await;
        let playlist_id = "1141084"; // Official Qobuz playlist
        client
            .get_playlist(playlist_id)
            .await
            .unwrap_or_else(|_| panic!("Couldn't  get playlist {playlist_id}"));
        client
            .get_playlist("no")
            .await
            .expect_err("There should be no playlist with id 'no'");
        // TODO: First  user playlist
    }

    #[test]
    async fn test_stream_track() {
        use futures::StreamExt;
        let track_id = "64868955";
        let mut stream = make_client()
            .await
            .stream_track(track_id, Quality::HiRes96)
            .await
            .unwrap_or_else(|_| panic!("Coudln't stream track with ID {track_id}"));
        assert!(stream.next().await.is_some());
    }
}
