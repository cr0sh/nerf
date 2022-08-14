mod futures;
mod spot;
mod tri_exchange;

pub use self::futures::*;
pub use spot::*;

use std::{
    convert::Infallible,
    fmt::{Debug, Write},
    future::Future,
    pin::Pin,
};

use chrono::{serde::ts_milliseconds, DateTime, Utc};
use hmac::{Hmac, Mac};
use hyper::body::Buf;
use nerf::{http::StatusCode, Client, HttpRequest, Request};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::Sha256;
use thiserror::Error;
use tracing::trace;

use crate::{
    common::{CommonOps, Unsupported},
    KeySecretAuthentication as Authentication,
};

use self::{__private::Sealed, tri_exchange::TriExchange};

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
    #[error("request to API server returned error, code: {code}, message: {msg}")]
    RequestFailed { code: i64, msg: String },
    #[error(transparent)]
    Hyper(#[from] hyper::Error),
    #[error("Unsupported HTTP method {0}")]
    UnsupportedHttpMethod(nerf::http::Method),
    /// A boxed error variant.
    /// [tower::buffer::Buffer] returns a Boxed error type so [Client]s must implement
    /// `From<Box<dyn StdError + Send + Sync + 'static>>` to support buffering.
    ///
    /// The conversion is done by manual downcasting to possible inner error variants
    /// and this variant is a fallback if every downcast fails.
    #[error(transparent)]
    Boxed(Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl From<Infallible> for Error {
    fn from(x: Infallible) -> Self {
        match x {}
    }
}

impl From<Box<dyn std::error::Error + Send + Sync + 'static>> for Error {
    fn from(x: Box<dyn std::error::Error + Send + Sync + 'static>) -> Self {
        let x = match x.downcast() {
            Ok(x) => return Self::Hyper(*x),
            Err(x) => x,
        };

        Self::Boxed(x)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderType {
    Limit,
    Market,
    StopLoss,
    StopLossLimit,
    TakeProfit,
    TakeProfitLimit,
    LimitMaker,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeInForce {
    #[serde(rename = "GTC")]
    GoodTilCanceled,
    #[serde(rename = "IOC")]
    ImmediateOrCancel,
    #[serde(rename = "FOK")]
    FillOrKill,
    #[serde(rename = "GTX")]
    GoodTilCrossing,
}

pub struct BinanceClient<S>(S);

impl<S> BinanceClient<S> {
    pub fn new(x: S) -> Self {
        Self(x)
    }

    pub fn with_auth(self, authentication: Authentication) -> BinancePrivateClient<S> {
        BinancePrivateClient {
            client: self,
            authentication,
        }
    }
}

pub struct BinancePrivateClient<S> {
    client: BinanceClient<S>,
    authentication: Authentication,
}

impl<T, S> Client<T> for BinanceClient<S>
where
    T: Request + HttpRequest + Sealed + Signer<Signer = Disabled> + Serialize + Debug,
    T::Response: DeserializeOwned,
{
    type Service = S;

    type Error = Error;

    type TryFromResponseFuture = Pin<Box<dyn Future<Output = Result<T::Response, Self::Error>>>>;

    fn service(&mut self) -> &mut Self::Service {
        &mut self.0
    }

    fn try_into_request(&mut self, x: T) -> Result<hyper::Request<hyper::Body>, Self::Error> {
        if x.method() == nerf::http::Method::GET {
            let params = serde_urlencoded::to_string(&x).map_err(Error::SerializeUrlencodedBody)?;
            let uri = x.uri();
            assert!(uri.query().is_none()); // TODO
            Ok(hyper::Request::builder()
                .uri(format!("{uri}?{params}"))
                .method(x.method())
                .body(hyper::Body::empty())
                .map_err(Error::ConstructHttpRequest)?)
        } else {
            let bytes = serde_json::to_vec(&x).map_err(Error::SerializeJsonBody)?;
            Ok(hyper::Request::builder()
                .uri(x.uri())
                .method(x.method())
                .body(bytes.into())
                .map_err(Error::ConstructHttpRequest)?)
        }
    }

    fn try_from_response(x: hyper::Response<hyper::Body>) -> Self::TryFromResponseFuture {
        Box::pin(async move {
            let status = x.status();
            let buf = hyper::body::aggregate(x).await?;
            if status != StatusCode::OK {
                #[derive(Deserialize)]
                struct ErrorResponse {
                    code: i64,
                    msg: String,
                }

                let error: ErrorResponse =
                    serde_json::from_reader(buf.reader()).map_err(Error::DeserializeJsonBody)?;
                Err(Error::RequestFailed {
                    code: error.code,
                    msg: error.msg,
                })
            } else {
                let resp =
                    serde_json::from_reader(buf.reader()).map_err(Error::DeserializeJsonBody)?;
                Ok(resp)
            }
        })
    }
}

impl<T, S> Client<T> for BinancePrivateClient<S>
where
    T: Request + HttpRequest + Sealed + Signer + Serialize + Debug,
    T::Response: DeserializeOwned,
{
    type Service = S;

    type Error = Error;

    type TryFromResponseFuture = Pin<Box<dyn Future<Output = Result<T::Response, Self::Error>>>>;

    fn service(&mut self) -> &mut Self::Service {
        &mut self.client.0
    }

    fn try_into_request(&mut self, x: T) -> Result<hyper::Request<hyper::Body>, Self::Error> {
        if !<T::Signer as SignerKind>::is_private() {
            if x.method() == nerf::http::Method::GET {
                let params =
                    serde_urlencoded::to_string(&x).map_err(Error::SerializeUrlencodedBody)?;
                let uri = x.uri();
                assert!(uri.query().is_none()); // TODO
                return hyper::Request::builder()
                    .uri(format!("{uri}?{params}"))
                    .method(x.method())
                    .body(hyper::Body::empty())
                    .map_err(Error::ConstructHttpRequest);
            } else {
                let bytes = serde_json::to_vec(&x).map_err(Error::SerializeJsonBody)?;
                return hyper::Request::builder()
                    .uri(x.uri())
                    .method(x.method())
                    .body(bytes.into())
                    .map_err(Error::ConstructHttpRequest);
            }
        }

        type HmacSha256 = Hmac<Sha256>;
        const SIGN_RECV_WINDOW_MILLIS: u64 = 5000;

        #[derive(Serialize, Debug)]
        #[serde(rename_all = "camelCase")]
        struct SignedRequest<R>
        where
            R: Serialize,
        {
            #[serde(flatten)]
            req: R,
            recv_window: u64,
            #[serde(with = "ts_milliseconds")]
            timestamp: DateTime<Utc>,
        }

        let req = x;
        let method = req.method();
        let uri = req.uri();
        let signed_req = SignedRequest {
            req,
            recv_window: SIGN_RECV_WINDOW_MILLIS,
            timestamp: chrono::Utc::now(),
        };
        trace!(uri = uri.to_string(), signed_req = ?signed_req, api_key = self.authentication.key(), method = method.to_string());
        let mut hmac = HmacSha256::new(self.authentication.secret().as_bytes().into());
        let params =
            serde_urlencoded::to_string(&signed_req).map_err(Error::SerializeUrlencodedBody)?;
        hmac.update(params.as_bytes());
        let signature = hmac.finalize().into_bytes();
        let signature = if params.is_empty() {
            let mut s = String::with_capacity(signature.len() * 2 + "signature=".len());
            s.push_str("signature=");
            for &b in signature.as_slice() {
                write!(&mut s, "{:02x}", b).unwrap();
            }
            s
        } else {
            let mut s = String::with_capacity(signature.len() * 2 + "&signature=".len());
            s.push_str("&signature=");
            for &b in signature.as_slice() {
                write!(&mut s, "{:02x}", b).unwrap();
            }
            s
        };

        let full_uri = format!("{uri}?{params}{signature}");
        trace!(full_uri = full_uri, method = ?method);
        if method == nerf::http::Method::GET {
            assert!(uri.query().is_none()); // TODO
            Ok(hyper::Request::builder()
                .uri(full_uri)
                .method(method)
                .header("X-MBX-APIKEY", self.authentication.key.clone())
                .body(hyper::Body::empty())
                .map_err(Error::ConstructHttpRequest)?)
        } else if method == nerf::http::Method::POST || method == nerf::http::Method::DELETE {
            Ok(hyper::Request::builder()
                .uri(full_uri)
                .method(method)
                .header("X-MBX-APIKEY", self.authentication.key.clone())
                .header("Content-Type", "x-www-form-urlencoded")
                .body(hyper::Body::empty())
                .map_err(Error::ConstructHttpRequest)?)
        } else {
            Err(Error::UnsupportedHttpMethod(method))
        }
    }

    fn try_from_response(x: hyper::Response<hyper::Body>) -> Self::TryFromResponseFuture {
        Box::pin(async move {
            let status = x.status();
            let buf = hyper::body::aggregate(x).await?;
            if status != StatusCode::OK {
                #[derive(Deserialize)]
                struct ErrorResponse {
                    code: i64,
                    msg: String,
                }

                let error: ErrorResponse =
                    serde_json::from_reader(buf.reader()).map_err(Error::DeserializeJsonBody)?;
                Err(Error::RequestFailed {
                    code: error.code,
                    msg: error.msg,
                })
            } else {
                let resp =
                    serde_json::from_reader(buf.reader()).map_err(Error::DeserializeJsonBody)?;
                Ok(resp)
            }
        })
    }
}

impl<S> CommonOps for BinanceClient<S> {
    type GetTradesRequest = TriExchange<GetApiV3Trades, GetFapiV1Trades, Unsupported>;

    type GetOrderbookRequest = TriExchange<GetApiV3Depth, GetFapiV1Depth, Unsupported>;

    type GetOrdersRequest = Unsupported;

    type GetAllOrdersRequest = Unsupported;

    type PlaceOrderRequest = Unsupported;

    type CancelOrderRequest = Unsupported;

    type CancelAllOrdersRequest = Unsupported;

    type GetBalanceRequest = Unsupported;

    type GetPositionRequest = Unsupported;
}

impl<S> tower::Service<Unsupported> for BinanceClient<S> {
    type Response = ::std::convert::Infallible;

    type Error = ::std::convert::Infallible;

    type Future = Unsupported;

    fn poll_ready(
        &mut self,
        _cx: &mut ::std::task::Context<'_>,
    ) -> ::std::task::Poll<Result<(), Self::Error>> {
        ::std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Unsupported) -> Self::Future {
        match req {}
    }
}

impl<S> CommonOps for BinancePrivateClient<S> {
    type GetTradesRequest = TriExchange<GetApiV3Trades, GetFapiV1Trades, Unsupported>;

    type GetOrderbookRequest = TriExchange<GetApiV3Depth, GetFapiV1Depth, Unsupported>;

    type GetOrdersRequest = TriExchange<GetApiV3OpenOrders, GetFapiV1OpenOrders, Unsupported>;

    type GetAllOrdersRequest = Unsupported; // FIXME: TriExchange requires ExtractMarketKind for a common request type

    type PlaceOrderRequest = TriExchange<PostApiV3Order, PostFapiV1Order, Unsupported>;

    type CancelOrderRequest = TriExchange<DeleteApiV3Orders, DeleteFapiV1Order, Unsupported>;

    type CancelAllOrdersRequest = Unsupported;

    type GetBalanceRequest = Unsupported; // FIXME: TriExchange requires ExtractMarketKind for a common request type

    type GetPositionRequest = GetFapiV2PositionRisk;
}

impl<S> tower::Service<Unsupported> for BinancePrivateClient<S> {
    type Response = ::std::convert::Infallible;

    type Error = ::std::convert::Infallible;

    type Future = Unsupported;

    fn poll_ready(
        &mut self,
        _cx: &mut ::std::task::Context<'_>,
    ) -> ::std::task::Poll<Result<(), Self::Error>> {
        ::std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Unsupported) -> Self::Future {
        match req {}
    }
}

trait Signer {
    type Signer: SignerKind;
}

struct Disabled;
struct UserDataSigner;

impl Signer for Unsupported {
    type Signer = Disabled;
}

trait SignerKind {
    fn is_private() -> bool;
}

impl SignerKind for Disabled {
    fn is_private() -> bool {
        false
    }
}

impl SignerKind for UserDataSigner {
    fn is_private() -> bool {
        true
    }
}

mod __private {
    use crate::common::Unsupported;

    pub trait Sealed {}
    impl Sealed for Unsupported {}
}
