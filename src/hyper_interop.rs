//! Interoperability between [`hyper::Client`]s.

use std::{future::Future, marker::PhantomData, pin::Pin};

use bytes::Bytes;
// To avoid ambiguity, avoid importing items under `hyper` namespace as possible.
use hyper::client::ResponseFuture;
use pin_project::pin_project;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tower::{Layer, Service};

use crate::{HttpRequest, Request};

#[derive(Error, Debug)]
pub enum HyperInteropError {
    #[error(transparent)]
    Super(#[from] crate::Error),
    #[error("Cannot construct HTTP request: {0}")]
    ConstructHttpRequest(#[from] http::Error),
    #[error("Cannot encode GET parameters into query: {0}")]
    UrlencodeGetParams(#[from] serde_urlencoded::ser::Error),
    #[error(transparent)]
    Hyper(#[from] hyper::Error),
}

/// Fallible conversion into [`hyper::Request<Body>`].
///
/// This is a workaround for 'orphan rule' which prevents using `impl` [`From<T>`] `for` [`hyper::Request<Body>`]
/// for such cases.
pub trait TryIntoHyperRequest {
    fn try_into_hyper_request(self) -> Result<hyper::Request<hyper::Body>, HyperInteropError>;
}

pub struct HyperLayer;

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
    Request: crate::Request<Response = Response> + TryIntoHyperRequest,
    Response: TryFrom<Bytes, Error = crate::Error>,
{
    type Response = Response;

    type Error = HyperInteropError;

    type Future = HyperInteropFuture<Response>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(From::from)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let req = match req.try_into_hyper_request() {
            Ok(x) => x,
            Err(err) => return HyperInteropFuture::SerializationFailed(Some(err)),
        };
        let fut = self.inner.call(req);
        HyperInteropFuture::SerializationSucceeded(HyperInteropFutureInner {
            response_fut: fut,
            to_bytes_fut: None,
            _phantom: PhantomData,
        })
    }
}

#[pin_project(project = HyperInteropFutureProj)]
pub enum HyperInteropFuture<Resp> {
    SerializationFailed(Option<HyperInteropError>),
    SerializationSucceeded(#[pin] HyperInteropFutureInner<Resp>),
}

#[pin_project]
pub struct HyperInteropFutureInner<Resp> {
    #[pin]
    response_fut: hyper::client::ResponseFuture,
    #[allow(clippy::type_complexity)]
    to_bytes_fut: Option<Pin<Box<dyn Future<Output = Result<Bytes, hyper::Error>>>>>,
    _phantom: PhantomData<Resp>,
}

impl<Resp> Future for HyperInteropFuture<Resp>
where
    Resp: TryFrom<Bytes, Error = crate::Error>,
{
    type Output = Result<Resp, HyperInteropError>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = match self.project() {
            HyperInteropFutureProj::SerializationSucceeded(this) => this.project(),
            HyperInteropFutureProj::SerializationFailed(err) => {
                assert!(err.is_some());
                return std::task::Poll::Ready(Err(err.take().unwrap()));
            }
        };
        if this.to_bytes_fut.is_none() {
            match this.response_fut.poll(cx) {
                std::task::Poll::Ready(x) => {
                    *this.to_bytes_fut = Some(Box::pin(hyper::body::to_bytes(x?)));
                }
                std::task::Poll::Pending => return std::task::Poll::Pending,
            }
        }
        match Pin::new(this.to_bytes_fut.as_mut().unwrap()).poll(cx) {
            std::task::Poll::Ready(Ok(bytes)) => {
                std::task::Poll::Ready(bytes.try_into().map_err(From::from))
            }
            std::task::Poll::Ready(Err(err)) => std::task::Poll::Ready(Err(err.into())),
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

impl<'a, T, Response> TryIntoHyperRequest for T
where
    T: HttpRequest + Request<Response = Response> + Serialize,
    Response: Deserialize<'a>,
{
    fn try_into_hyper_request(self) -> Result<hyper::Request<hyper::Body>, HyperInteropError> {
        if self.method() == http::Method::GET {
            let params = serde_urlencoded::to_string(&self)
                .map_err(HyperInteropError::UrlencodeGetParams)?;
            let uri = self.uri();
            assert!(uri.query().is_none()); // TODO
            Ok(hyper::Request::builder()
                .uri(format!("{uri}?{params}"))
                .method(self.method())
                .body(hyper::Body::empty())?)
        } else {
            let bytes = serde_json::to_vec(&self).map_err(crate::Error::SerializeRequest)?;
            Ok(hyper::Request::builder()
                .uri(self.uri())
                .method(self.method())
                .body(bytes.into())?)
        }
    }
}
