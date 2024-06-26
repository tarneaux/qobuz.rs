use std::error::Error;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fmt, fmt::Display, fmt::Formatter};

use crate::quality::Quality;

const API_URL: &str = "https://www.qobuz.com/api.json/0.2/";
const API_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:83.0) Gecko/20100101 Firefox/83.0";

#[derive(Debug)]
pub struct Client {
    pub client: reqwest::Client,
    secret: Option<String>,
}

impl Client {
    pub async fn new(
        email: &str,
        pwd: &str,
        app_id: &str,
        secrets: Vec<String>,
    ) -> Result<Self, Box<dyn Error>> {
        let client = make_http_client(app_id, None);
        let mut api_client = Self {
            client,
            secret: None,
        };
        api_client.login(email, pwd, secrets, app_id).await?;
        println!("Secret: {}", api_client.secret.clone().unwrap());
        Ok(api_client)
    }

    async fn login(
        &mut self,
        email: &str,
        pwd: &str,
        secrets: Vec<String>,
        app_id: &str,
    ) -> Result<(), LoginError> {
        let params = [("email", email), ("password", pwd), ("app_id", &app_id)];
        let resp = self
            .do_request("user/login", &params)
            .await
            .map_err(|e| match e.status() {
                Some(reqwest::StatusCode::UNAUTHORIZED) => LoginError::InvalidCredentials,
                Some(reqwest::StatusCode::BAD_REQUEST) => LoginError::InvalidAppId,
                _ => LoginError::ReqwestError(e),
            })?;
        // verify json["user"]["credential"]["parameters"] exists.
        // If not, we are authenticating into a free account which can't download tracks.
        // TODO: find a way to check without unwrap's.
        println!(
            "{}",
            resp.get("user")
                .unwrap()
                .get("credential")
                .unwrap()
                .get("parameters")
                .unwrap()
        );
        match resp.get("user_auth_token") {
            Some(Value::String(token)) => {
                self.set_correct_secret(secrets).await?;
                println!("{}", token);
                self.client = make_http_client(&app_id, Some(token));
                Ok(())
            }
            None | Some(_) => Err(LoginError::NoUserAuthToken),
        }
    }

    pub async fn get_track_file_url(
        &self,
        track_id: &str,
        quality: Quality,
    ) -> Result<DownloadUrl, GetDownloadUrlError> {
        let timestamp_now = chrono::Utc::now().timestamp().to_string();

        let quality_id: u8 = quality.into();

        let r_sig = format!(
            "trackgetFileUrlformat_id{}intentstreamtrack_id{}{}{}",
            quality_id,
            track_id,
            timestamp_now,
            self.secret.as_ref().unwrap()
        );

        println!("{r_sig}");

        let r_sig_hash = format!("{:x}", md5::compute(r_sig));
        println!("Hash: {r_sig_hash}");

        let params = [
            ("request_ts", timestamp_now.as_str()),
            ("request_sig", &r_sig_hash),
            ("track_id", track_id),
            ("format_id", &quality_id.to_string()),
            ("intent", "stream"),
        ];
        let res = self.do_request("track/getFileUrl", &params).await?;
        if let Some(Value::Bool(true)) = res.get("sample") {
            return Err(GetDownloadUrlError::IsSample);
        }
        let res: DownloadUrl = serde_json::from_value(res)?;
        Ok(res)
    }

    async fn do_request(
        &self,
        path: &str,
        params: &[(&str, &str)],
    ) -> Result<Value, reqwest::Error> {
        let url = format!("{API_URL}{path}");
        let resp = self
            .client
            .get(&url)
            .query(params)
            .send()
            .await?
            .error_for_status()?;
        let json: Value = resp.json().await?;
        Ok(json)
    }

    async fn set_correct_secret(&mut self, secrets: Vec<String>) -> Result<(), LoginError> {
        for secret in secrets {
            if secret.is_empty() {
                continue;
            }
            self.secret = Some(secret);
            match self.get_track_file_url("64868958", Quality::HiRes192).await {
                Err(GetDownloadUrlError::IsSample) => return Ok(()),
                Err(GetDownloadUrlError::Reqwest(e)) => {
                    e.status().expect(&format!(
                        "Error while getting correct secret: returned error is unexpected {e}"
                    ));
                    continue;
                }
                Err(e) => return Err(LoginError::GetDownloadUrlError(e)),
                // Since the X-User-Auth-Token header isn't set yet, we can't get a non-sample URL.
                Ok(_) => unreachable!(),
            }
        }
        self.secret = None;
        Err(LoginError::NoValidSecret)
    }
}

#[derive(Debug)]
pub enum LoginError {
    InvalidCredentials,
    InvalidAppId,
    ReqwestError(reqwest::Error),
    NoUserAuthToken,
    NoValidSecret,
    GetDownloadUrlError(GetDownloadUrlError),
}
impl Display for LoginError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            LoginError::InvalidCredentials => write!(f, "Invalid credentials"),
            LoginError::InvalidAppId => write!(f, "Invalid app id"),
            LoginError::ReqwestError(e) => write!(f, "Reqwest error: {}", e),
            LoginError::NoUserAuthToken => write!(f, "No user auth token"),
            LoginError::NoValidSecret => write!(f, "No valid secret found"),
            LoginError::GetDownloadUrlError(e) => write!(
                f,
                "Error while trying to get download URL to test token: {e}"
            ),
        }
    }
}
impl Error for LoginError {}

#[derive(Debug)]
pub enum GetDownloadUrlError {
    IsSample,
    SerdeJson(serde_json::Error),
    Reqwest(reqwest::Error),
}
impl Display for GetDownloadUrlError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::IsSample => write!(f, "Downloadable file is a sample"),
            Self::SerdeJson(e) => write!(f, "Serde error: {e}"),
            Self::Reqwest(e) => write!(f, "Reqwest error: {e}"),
        }
    }
}
impl Error for GetDownloadUrlError {}
impl From<serde_json::Error> for GetDownloadUrlError {
    fn from(value: serde_json::Error) -> Self {
        Self::SerdeJson(value)
    }
}
impl From<reqwest::Error> for GetDownloadUrlError {
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
