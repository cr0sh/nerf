//! Interoperability between [`hyper::Client`]s.

use std::{future::Future, marker::PhantomData, task::Poll};

// To avoid ambiguity, avoid importing items under `hyper` namespace as possible.
use hyper::client::ResponseFuture;
use pin_project::pin_project;
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
    type Service = HyperInteropService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        HyperInteropService { inner }
    }
}

/// Wrapped [`hyper::Client`] to process nerf requests.
pub struct HyperInteropService<S> {
    inner: S,
}

impl<S, Request, Response> Service<Request> for HyperInteropService<S>
where
    // Don't panic: this means S is either `hyper::Client` or `&hyper::Client`
    S: Service<
        hyper::Request<hyper::Body>,
        Response = hyper::Response<hyper::Body>,
        Error = hyper::Error,
        Future = ResponseFuture,
    >,
    Request: crate::Request<Response = Response> + TryInto<hyper::Request<hyper::Body>>,
    hyper::Response<hyper::Body>: TryIntoResponse<Response, Error = Request::Error>,
{
    type Response = Response;

    type Error = HyperInteropError<Request::Error>;

    type Future = HyperInteropFuture<
        Response,
        Request::Error,
        <hyper::Response<hyper::Body> as TryIntoResponse<Response>>::Future,
    >;

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
                return HyperInteropFuture::RequestConversionFailed(Some(
                    HyperInteropError::ConversionFailed(err),
                ))
            }
        };
        HyperInteropFuture::AwaitingResponse(self.inner.call(req), PhantomData)
    }
}

#[pin_project(project = HyperInteropFutureProj)]
pub enum HyperInteropFuture<Resp, E, TryIntoFut> {
    RequestConversionFailed(Option<HyperInteropError<E>>),
    AwaitingResponse(#[pin] hyper::client::ResponseFuture, PhantomData<Resp>),
    ConvertingResponse(#[pin] TryIntoFut),
}

impl<Resp, E, TryIntoFut> Future for HyperInteropFuture<Resp, E, TryIntoFut>
where
    hyper::Response<hyper::Body>: TryIntoResponse<Resp, Future = TryIntoFut, Error = E>,
    TryIntoFut: Future<
        Output = Result<Resp, <hyper::Response<hyper::Body> as TryIntoResponse<Resp>>::Error>,
    >,
{
    type Output = Result<Resp, HyperInteropError<E>>;

    #[tracing::instrument(skip_all)]
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        loop {
            match self.as_mut().project() {
                HyperInteropFutureProj::AwaitingResponse(fut, _) => {
                    let result = match fut.poll(cx)? {
                        Poll::Ready(x) => x,
                        Poll::Pending => return Poll::Pending,
                    };

                    self.set(HyperInteropFuture::ConvertingResponse(
                        result.try_into_response(),
                    ));
                }
                HyperInteropFutureProj::RequestConversionFailed(err) => {
                    assert!(err.is_some());
                    return std::task::Poll::Ready(Err(err.take().unwrap()));
                }
                HyperInteropFutureProj::ConvertingResponse(fut) => {
                    return fut.poll(cx).map_err(HyperInteropError::ConversionFailed)
                }
            };
        }
    }
}
