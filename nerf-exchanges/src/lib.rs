use std::fmt::Debug;

pub mod binance;
pub mod common;
pub mod upbit;

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
