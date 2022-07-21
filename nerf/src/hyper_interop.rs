//! Interoperability between [`hyper::Client`]s.

use std::{convert::Infallible, future::Future, pin::Pin};

// To avoid ambiguity, avoid importing items under `hyper` namespace as possible.
use thiserror::Error;
use tower::{Layer, Service};

use crate::TryIntoResponse;

#[derive(Error, Debug)]
pub enum HyperInteropError<E> {
    #[error(transparent)]
    Super(#[from] crate::Error),
    #[error(transparent)]
    Hyper(#[from] hyper::Error),
    #[error("server responded with non-OK code {0}, UTF-8(lossy) contents: {1}")]
    RequestFailed(http::StatusCode, String),
    #[error(transparent)]
    ConversionFailed(E), // Conversion of request or response failed
}

impl<E> From<Infallible> for HyperInteropError<E> {
    fn from(x: Infallible) -> Self {
        match x {}
    }
}

pub struct HyperLayer(());

impl HyperLayer {
    pub fn new() -> Self {
        Self(())
    }
}

impl Default for HyperLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Layer<S> for HyperLayer {
    type Service = HyperInteropService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        HyperInteropService { inner }
    }
}

/// Wrapped [`hyper::Client`] to process nerf requests.
pub struct HyperInteropService<S> {
    inner: S,
}

impl<S, Request, Response, ResponseFuture> Service<Request> for HyperInteropService<S>
where
    // Don't panic: this means S is either `hyper::Client` or `&hyper::Client`
    S: Service<
        hyper::Request<hyper::Body>,
        Response = hyper::Response<hyper::Body>,
        Error = hyper::Error,
        Future = ResponseFuture,
    >,
    Request: crate::Request<Response = Response> + TryInto<hyper::Request<hyper::Body>>,
    <Request as std::convert::TryInto<http::Request<hyper::Body>>>::Error: 'static,
    hyper::Response<hyper::Body>: TryIntoResponse<Response, Error = Request::Error>,
    ResponseFuture: Future<Output = Result<<S as Service<hyper::Request<hyper::Body>>>::Response, hyper::Error>>
        + 'static,
{
    type Response = Response;

    type Error = HyperInteropError<Request::Error>;

    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;
    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(From::from)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let req = match req.try_into().map_err(HyperInteropError::ConversionFailed) {
            Ok(x) => x,
            Err(e) => return Box::pin(async move { Err(e) }),
        };
        let fut = self.inner.call(req);

        Box::pin(async move {
            fut.await
                .map_err(HyperInteropError::Hyper)?
                .try_into_response()
                .await
                .map_err(HyperInteropError::ConversionFailed)
        })
    }
}
