//! Interoperability between [`hyper::Client`]s.

use std::{future::Future, marker::PhantomData, pin::Pin};

use bytes::Bytes;
// To avoid ambiguity, avoid importing items under `hyper` namespace as possible.
use hyper::client::ResponseFuture;
use pin_project::pin_project;
use thiserror::Error;
use tower::{Layer, Service};
use tracing::debug;

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

impl<S> Layer<S> for HyperLayer
where
    S: Service<hyper::Request<hyper::Body>, Response = hyper::Response<hyper::Body>>,
{
    type Service = HyperInterop<S>;

    fn layer(&self, inner: S) -> Self::Service {
        HyperInterop { inner }
    }
}

/// Wrapped [`hyper::Client`] to process nerf requests.
pub struct HyperInterop<S> {
    inner: S,
}

impl<S, Request, Response> Service<Request> for HyperInterop<S>
where
    // Don't panic: this means S is either `hyper::Client` or `&hyper::Client`
    S: Service<
        hyper::Request<hyper::Body>,
        Response = hyper::Response<hyper::Body>,
        Error = hyper::Error,
        Future = ResponseFuture,
    >,
    Request: crate::Request<Response = Response> + TryInto<hyper::Request<hyper::Body>>,
    Response: TryFrom<Bytes, Error = Request::Error>,
{
    type Response = Response;

    type Error = HyperInteropError<Request::Error>;

    type Future = HyperInteropFuture<Response, Request::Error>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(From::from)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let req = match req.try_into() {
            Ok(x) => x,
            Err(err) => {
                return HyperInteropFuture::ConversionFailed(Some(
                    HyperInteropError::ConversionFailed(err),
                ))
            }
        };
        let fut = self.inner.call(req);
        HyperInteropFuture::ConversionSucceeded(HyperInteropFutureInner {
            response_fut: fut,
            to_bytes_fut: None,
            response_status: None,
            _phantom: PhantomData,
        })
    }
}

#[pin_project(project = HyperInteropFutureProj)]
pub enum HyperInteropFuture<Resp, E> {
    ConversionFailed(Option<HyperInteropError<E>>),
    ConversionSucceeded(#[pin] HyperInteropFutureInner<Resp>),
}

#[pin_project]
pub struct HyperInteropFutureInner<Resp> {
    #[pin]
    response_fut: hyper::client::ResponseFuture,
    #[allow(clippy::type_complexity)]
    to_bytes_fut: Option<Pin<Box<dyn Future<Output = Result<Bytes, hyper::Error>>>>>,
    response_status: Option<http::StatusCode>,
    _phantom: PhantomData<Resp>,
}

impl<Resp, E> Future for HyperInteropFuture<Resp, E>
where
    Resp: TryFrom<Bytes, Error = E>,
{
    type Output = Result<Resp, HyperInteropError<E>>;

    #[tracing::instrument(skip_all)]
    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = match self.project() {
            HyperInteropFutureProj::ConversionSucceeded(this) => this.project(),
            HyperInteropFutureProj::ConversionFailed(err) => {
                assert!(err.is_some());
                return std::task::Poll::Ready(Err(err.take().unwrap()));
            }
        };
        if this.to_bytes_fut.is_none() {
            match this.response_fut.poll(cx) {
                std::task::Poll::Ready(Ok(resp)) => {
                    debug!(response_status = resp.status().to_string());
                    *this.response_status = Some(resp.status());
                    *this.to_bytes_fut = Some(Box::pin(hyper::body::to_bytes(resp)));
                }
                std::task::Poll::Ready(Err(err)) => {
                    return std::task::Poll::Ready(Err(HyperInteropError::Hyper(err)))
                }
                std::task::Poll::Pending => return std::task::Poll::Pending,
            }
        }
        match Pin::new(this.to_bytes_fut.as_mut().unwrap()).poll(cx) {
            std::task::Poll::Ready(Ok(bytes)) => {
                if this.response_status.unwrap() != http::StatusCode::OK {
                    return std::task::Poll::Ready(Err(HyperInteropError::RequestFailed(
                        this.response_status.unwrap(),
                        String::from_utf8_lossy(&bytes).to_string(),
                    )));
                }
                std::task::Poll::Ready(
                    bytes
                        .try_into()
                        .map_err(HyperInteropError::ConversionFailed),
                )
            }
            std::task::Poll::Ready(Err(err)) => std::task::Poll::Ready(Err(err.into())),
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}
