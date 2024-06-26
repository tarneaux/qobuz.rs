use std::error::Error;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fmt, fmt::Display, fmt::Formatter};

const API_URL: &str = "https://www.qobuz.com/api.json/0.2/";
const API_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:83.0) Gecko/20100101 Firefox/83.0";

#[derive(Debug)]
pub struct ApiClient {
    pub client: reqwest::Client,
    app_id: String,
    secret: Option<String>,
}

impl ApiClient {
    pub async fn new(
        email: &str,
        pwd: &str,
        app_id: &str,
        secrets: Vec<String>,
    ) -> Result<Self, Box<dyn Error>> {
        let client = make_http_client(app_id, None);
        let mut api_client = Self {
            client,
            app_id: app_id.to_string(),
            secret: None,
        };
        api_client.login(email, pwd, secrets).await?;
        println!("Secret: {}", api_client.secret.clone().unwrap());
        Ok(api_client)
    }

    async fn login(
        &mut self,
        email: &str,
        pwd: &str,
        secrets: Vec<String>,
    ) -> Result<(), LoginError> {
        let params = [
            ("email", email),
            ("password", pwd),
            ("app_id", &self.app_id),
        ];
        let url = format!("{}{}", API_URL, "user/login");
        let resp = self
            .client
            .get(&url)
            .query(&params)
            .send()
            .await
            .map_err(LoginError::ReqwestError)?;
        match resp.status() {
            reqwest::StatusCode::OK => Ok(()),
            reqwest::StatusCode::UNAUTHORIZED => Err(LoginError::InvalidCredentials),
            reqwest::StatusCode::BAD_REQUEST => Err(LoginError::InvalidAppId),
            _ => Err(LoginError::UnknownError),
        }?;
        let json: Value = resp.json().await.map_err(LoginError::ReqwestError)?;
        // verify json["user"]["credential"]["parameters"] exists.
        // If not, we are authenticating into a free account which can't download tracks.
        // TODO: find a way to check without unwrap's.
        println!(
            "{}",
            json.get("user")
                .unwrap()
                .get("credential")
                .unwrap()
                .get("parameters")
                .unwrap()
        );
        match json.get("user_auth_token") {
            Some(Value::String(token)) => {
                self.set_correct_secret(secrets).await?;
                println!("{}", token);
                self.client = make_http_client(&self.app_id, Some(token));
                Ok(())
            }
            None | Some(_) => Err(LoginError::NoUserAuthToken),
        }
    }

    pub async fn get_track_file_url(
        &self,
        track_id: &str,
    ) -> Result<Option<String>, Box<dyn Error>> {
        let timestamp_now = chrono::Utc::now().timestamp().to_string();
        let fmt_id = "6"; // TODO: add an option for this. Should be 5, 6, 7 or 27.

        let r_sig = format!(
            "trackgetFileUrlformat_id{}intentstreamtrack_id{}{}{}",
            fmt_id,
            track_id,
            timestamp_now,
            self.secret.as_ref().unwrap()
        );

        println!("{r_sig}");

        let r_sig_hash = format!("{:?}", md5::compute(r_sig));
        println!("Hash: {r_sig_hash}");

        let params = [
            ("request_ts", timestamp_now.as_str()),
            ("request_sig", &r_sig_hash),
            ("track_id", track_id),
            ("format_id", fmt_id),
            ("intent", "stream"),
        ];
        let res: GetUrlResponse =
            serde_json::from_value(self.do_request("track/getFileUrl", &params).await?)?;
        if res.sample {
            return Ok(None);
        }
        Ok(Some(res.url))
    }

    async fn do_request(
        &self,
        path: &str,
        params: &[(&str, &str)],
    ) -> Result<Value, Box<dyn Error>> {
        let url = format!("{API_URL}{path}");
        let resp = self
            .client
            .get(&url)
            .query(params)
            .send()
            .await
            .map_err(|e| format!("Reqwest error: {}", e))?;
        if resp.status() == reqwest::StatusCode::BAD_REQUEST {
            return Err(format!("Invalid app secret: {}", self.app_id).into());
        }
        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err("Unauthorized".into());
        }
        let json: Value = resp
            .json()
            .await
            .map_err(|e| format!("Reqwest error: {}", e))?;
        Ok(json)
    }

    async fn set_correct_secret(&mut self, secrets: Vec<String>) -> Result<(), LoginError> {
        for secret in secrets {
            if secret.is_empty() {
                continue;
            }
            self.secret = Some(secret);
            match self.get_track_file_url("64868958").await {
                Ok(_) => {
                    return Ok(());
                }
                Err(_) => {
                    println!("A secret didn't work.");
                    continue;
                }
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
    UnknownError,
    NoValidSecret,
}

impl Display for LoginError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            LoginError::InvalidCredentials => write!(f, "Invalid credentials"),
            LoginError::InvalidAppId => write!(f, "Invalid app id"),
            LoginError::ReqwestError(e) => write!(f, "Reqwest error: {}", e),
            LoginError::NoUserAuthToken => write!(f, "No user auth token"),
            LoginError::UnknownError => write!(f, "Unknown error"),
            LoginError::NoValidSecret => write!(f, "No valid secret found"),
        }
    }
}

impl Error for LoginError {}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct GetUrlResponse {
    #[serde(default = "default_sample")]
    sample: bool,
    bit_depth: u16, // TODO: object for bit depth
    // sampling_rate
    url: String,
}

const fn default_sample() -> bool {
    false
}

fn make_http_client(app_id: &str, uat: Option<&str>) -> Client {
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
