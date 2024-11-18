use bytes::Bytes;
use futures::Stream;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::env::VarError;
use std::error::Error;
use std::{env, fmt, fmt::Display, fmt::Formatter};

use crate::{
    extra, quality::Quality, Album, Array, Artist, Playlist, PlaylistWithExtra, QobuzType, Track,
};

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
        if let Some(Value::Bool(true)) = res.get("sample") {
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
    pub async fn get_user_favorites<T: QobuzType>(&self) -> Result<Vec<T>, ApiError> {
        let timestamp_now = chrono::Utc::now().timestamp().to_string();
        let r_sig_hash = format!(
            "{:x}",
            md5::compute(format!(
                "favoritegetUserFavorites{timestamp_now}{}",
                self.secret
            ))
        );
        let fav_type = T::name_plural();
        let params = [
            ("type", fav_type),
            ("request_ts", &timestamp_now),
            ("request_sig", &r_sig_hash),
            ("limit", "500"),
            ("offset", "0"), // TODO: walk
        ];
        let res: Value = self
            .do_request("favorite/getUserFavorites", &params)
            .await?;
        Ok(serde_json::from_value::<Array<T>>(
            res.get(fav_type)
                .unwrap_or_else(|| panic!("Couldn't get '{fav_type}' field from returned data while getting user favorites"))
                .clone(),
        )?.items)
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
    /// let mut artist = client
    ///     .get_track("129342731")
    ///     .await
    ///     .unwrap();
    /// # })
    /// ```
    pub async fn get_track(
        &self,
        track_id: &str,
    ) -> Result<Track<extra::AlbumAndComposer>, ApiError> {
        let params = [("track_id", track_id)];
        let res = self.do_request("track/get", &params).await?;
        Ok(serde_json::from_value(res)?)
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
    /// let mut artist = client
    ///     .get_playlist("1141084")
    ///     .await
    ///     .unwrap();
    /// # })
    /// ```
    pub async fn get_playlist(&self, playlist_id: &str) -> Result<Playlist, ApiError> {
        let count = self
            .do_request::<PlaylistWithExtra<extra::Tracks>>(
                "playlist/get",
                &[
                    ("playlist_id", playlist_id),
                    ("extra", "tracks"),
                    ("limit", "1"),
                    ("offset", "0"),
                ],
            )
            .await?
            .extra
            .tracks
            .total;
        // TODO: Global configuration for track count ?
        self.do_request(
            "playlist/get",
            &[
                ("playlist_id", playlist_id),
                ("extra", "tracks"),
                ("limit", &count.to_string()),
                ("offset", "0"),
            ],
        )
        .await
        .map_err(|e| e.into())
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
    /// let mut artist = client
    ///     .get_album("trrcz9pvaaz6b")
    ///     .await
    ///     .unwrap();
    /// # })
    /// ```
    pub async fn get_album(&self, album_id: &str) -> Result<Album<extra::Tracks>, ApiError> {
        self.do_request("album/get", &[("album_id", album_id)])
            .await
            .map_err(|e| e.into())
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
    /// let mut artist = client
    ///     .get_artist("26390")
    ///     .await
    ///     .unwrap();
    /// # })
    /// ```
    pub async fn get_artist(&self, artist_id: &str) -> Result<Artist<extra::Albums>, ApiError> {
        let count = self
            .do_request::<Artist<extra::Albums>>(
                "artist/get",
                &[
                    ("artist_id", artist_id),
                    ("extra", "albums"),
                    ("limit", "1"),
                    ("offset", "0"),
                ],
            )
            .await?
            .extra
            .albums
            .total;
        // TODO: Global configuration for track count ?
        self.do_request(
            "artist/get",
            &[
                ("artist_id", artist_id),
                ("extra", "albums"),
                ("limit", &count.to_string()),
                ("offset", "0"),
            ],
        )
        .await
        .map_err(|e| e.into())
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

#[derive(Debug)]
pub enum LoginError {
    InvalidCredentials,
    InvalidAppId,
    ReqwestError(reqwest::Error),
    NoUserAuthToken,
    FreeAccount,
}
impl Display for LoginError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            LoginError::InvalidCredentials => write!(f, "Invalid credentials"),
            LoginError::InvalidAppId => write!(f, "Invalid app id"),
            LoginError::ReqwestError(e) => write!(f, "Reqwest error: {}", e),
            LoginError::NoUserAuthToken => write!(f, "No user auth token"),
            LoginError::FreeAccount => write!(
                f,
                "Tried to authenticate into a free account, which can't download tracks."
            ),
        }
    }
}
impl Error for LoginError {}

#[derive(Debug)]
pub enum ApiError {
    IsSample,
    SerdeJson(serde_json::Error),
    Reqwest(reqwest::Error),
}
impl Display for ApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::IsSample => write!(f, "Downloadable file is a sample"),
            Self::SerdeJson(e) => write!(f, "Serde error: {e}"),
            Self::Reqwest(e) => write!(f, "Reqwest error: {e}"),
        }
    }
}
impl Error for ApiError {}
impl From<serde_json::Error> for ApiError {
    fn from(value: serde_json::Error) -> Self {
        Self::SerdeJson(value)
    }
}
impl From<reqwest::Error> for ApiError {
    fn from(value: reqwest::Error) -> Self {
        Self::Reqwest(value)
    }
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
        "application/json;charset=UTF-8".parse().unwrap(),
    );
    if let Some(token) = uat {
        headers.insert("X-User-Auth-Token", token.parse().unwrap());
    }
    reqwest::ClientBuilder::new()
        .user_agent(API_USER_AGENT)
        .default_headers(headers)
        .build()
        .unwrap()
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
            e.status().unwrap_or_else(|| {
                panic!("Error while getting correct secret: returned error is unexpected {e}")
            });
            Ok(false)
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
    use tokio::test;

    pub async fn make_client() -> Client {
        let credentials = QobuzCredentials::from_env()
            .expect("Couldn't get credentials env variables which need to be set for this test.");
        Client::new(credentials).await.unwrap()
    }

    #[test]
    async fn test_get_user_favorites() {
        let client = make_client().await;
        client.get_user_favorites::<Album<()>>().await.unwrap();
        client.get_user_favorites::<Track<()>>().await.unwrap();
        client.get_user_favorites::<Artist<()>>().await.unwrap();
    }

    #[test]
    async fn test_get_track_file_url() {
        make_client()
            .await
            .get_track_file_url("64868955", Quality::HiRes96)
            .await
            .unwrap();
    }

    #[test]
    async fn test_get_track() {
        let client = make_client().await;
        client.get_track("64868955").await.unwrap();
        client.get_track("no").await.unwrap_err();
    }

    #[test]
    async fn test_get_album() {
        let client = make_client().await;
        client.get_album("trrcz9pvaaz6b").await.unwrap();
        client.get_album("no").await.unwrap_err();
    }

    #[test]
    async fn test_get_artist() {
        let client = make_client().await;
        client.get_artist("26390").await.unwrap();
        client.get_artist("no").await.unwrap_err();
    }

    #[test]
    async fn test_get_playlist() {
        let client = make_client().await;
        client.get_playlist("1141084").await.unwrap(); // Official Qobuz playlist
        client.get_playlist("no").await.unwrap_err();
        // TODO: First  user playlist
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
