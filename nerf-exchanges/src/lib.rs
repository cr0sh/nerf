pub mod binance;
pub mod common;

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
