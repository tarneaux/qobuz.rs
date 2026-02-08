//! Qobuz API authentication.

use super::{do_request, make_http_client};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::env::VarError;
use thiserror::Error;

/// Credentials for Qobuz.
///
/// Use qobuz-dl to get these.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Credentials {
    pub email: String,
    /// Hashed password.
    pub password: String,
    pub app_id: String,
    pub secret: String,
}

impl Credentials {
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

pub(super) async fn get_user_auth_token(credentials: &Credentials) -> Result<String, LoginError> {
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
    if resp
        .get("user")
        .and_then(|v| v.get("credential"))
        .and_then(|v| v.get("parameters"))
        .is_none()
    {
        return Err(LoginError::FreeAccount);
    }
    match resp.get("user_auth_token") {
        Some(Value::String(uat)) => Ok(uat.to_string()),
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
