#![warn(clippy::print_stderr, clippy::print_stdout)]
//! nerf is a toolkit to create client-side SDK for (mainly) HTTP endpoint APIs.

mod error;
mod hyper_interop;
pub use hyper_interop::HyperLayer;

pub use bytes::Bytes;
pub use error::Error;
pub use http;
pub use nerf_macros::rate_limited;
pub use serde;
pub use serde_json;

/// Rate limit with weights.
pub trait WeightedRateLimit {
    fn weight(&self) -> u64;
}

/// Request/response pair.
pub trait Request {
    /// 'Expected' response type. Error case should not be included here.
    /// TODO: introduce how to handle errors with `tower::Layer`
    type Response;
}

/// HTTP request metadata.
pub trait HttpRequest {
    /// HTTP method of this request.
    fn method(&self) -> http::Method;
    fn uri(&self) -> http::Uri;
}
