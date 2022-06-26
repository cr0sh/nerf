use thiserror::Error;

/// Common errors across the crate.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    #[error("Cannot serialize request into JSON bytes: {0}")]
    SerializeRequest(serde_json::Error),
    #[error("Cannot deserialize response into JSON bytes: {0}")]
    DeserializeResponse(serde_json::Error),
}
