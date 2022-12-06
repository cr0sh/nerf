use std::fmt::Debug;

use thiserror::Error;

pub mod binance;
pub mod common;
pub mod upbit;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot serialize request body into JSON: {0}")]
    SerializeJsonBody(serde_json::Error),
    #[error("cannot serialize request to URL-encoded parameters: {0}")]
    SerializeUrlencodedBody(serde_urlencoded::ser::Error),
    #[error("cannot construct http::Request: {0}")]
    ConstructHttpRequest(nerf::http::Error),
    #[error("cannot deserialize response into JSON: {0}")]
    DeserializeJsonBody(serde_json::Error),
    #[error("request to API server returned error, code: {code:?}, message: {msg:?}")]
    RequestFailed {
        code: Option<String>,
        msg: Option<String>,
    },
    #[error(transparent)]
    Hyper(#[from] hyper::Error),
    #[error("cannot sign JWT payload for authentication: {0}")]
    Jwt(jwt::Error),
    #[error("Unsupported HTTP method {0}")]
    UnsupportedHttpMethod(nerf::http::Method),
}

#[derive(Clone)]
pub struct KeySecretAuthentication {
    key: String,
    secret: String,
}

impl KeySecretAuthentication {
    pub fn new(key: &str, secret: &str) -> Self {
        Self {
            key: key.to_string(),
            secret: secret.to_string(),
        }
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn secret(&self) -> &str {
        &self.secret
    }
}

impl Debug for KeySecretAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeySecretAuthentication")
            .field("key", &Box::new("<redacted>"))
            .field("secret", &Box::new("redacted"))
            .finish()
    }
}
