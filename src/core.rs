use std::error::Error;

use serde_json::Value;
use std::{fmt, fmt::Display, fmt::Formatter};

const API_URL: &str = "https://www.qobuz.com/api.json/0.2/";
const API_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:83.0) Gecko/20100101 Firefox/83.0";

#[derive(Debug)]
pub struct ApiClient {
    client: reqwest::Client,
    app_id: String,
    user_auth_token: Option<String>,
}

impl ApiClient {
    pub async fn new(email: &str, pwd: &str, app_id: &str) -> Result<Self, Box<dyn Error>> {
        let client = reqwest::ClientBuilder::new()
            .user_agent(API_USER_AGENT)
            .build()
            .unwrap();
        let mut api_client = Self {
            client,
            app_id: app_id.to_string(),
            user_auth_token: None,
        };
        api_client.login(email, pwd).await?;
        Ok(api_client)
    }

    async fn login(&mut self, email: &str, pwd: &str) -> Result<(), LoginError> {
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
        // TODO: verify json["user"]["credential"]["parameters"] exists.
        // If not, we are authenticating into a free account which can't download tracks.
        match json.get("user_auth_token") {
            Some(token) => {
                self.user_auth_token = Some(token.as_str().unwrap().to_string());
                Ok(())
            }
            None => Err(LoginError::NoUserAuthToken),
        }
    }
}

#[derive(Debug)]
pub enum LoginError {
    InvalidCredentials,
    InvalidAppId,
    ReqwestError(reqwest::Error),
    NoUserAuthToken,
    UnknownError,
}

impl Display for LoginError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            LoginError::InvalidCredentials => write!(f, "Invalid credentials"),
            LoginError::InvalidAppId => write!(f, "Invalid app id"),
            LoginError::ReqwestError(e) => write!(f, "Reqwest error: {}", e),
            LoginError::NoUserAuthToken => write!(f, "No user auth token"),
            LoginError::UnknownError => write!(f, "Unknown error"),
        }
    }
}

impl Error for LoginError {}
