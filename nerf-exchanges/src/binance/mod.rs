mod futures;
mod spot;

pub use self::futures::*;
pub use spot::*;

use std::{
    fmt::{Debug, Write},
    future::Future,
    pin::Pin,
};

use chrono::{serde::ts_milliseconds, DateTime, Utc};
use hmac::{Hmac, Mac};
use hyper::body::Buf;
use nerf::{http::StatusCode, HttpRequest, Request};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::Sha256;
use tracing::trace;

use crate::{
    common::{Signer, SignerKind},
    Error, KeySecretAuthentication,
};

use self::__private::Sealed;

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

fn try_into_request<T>(x: T) -> Result<hyper::Request<hyper::Body>, Error>
where
    T: Request + HttpRequest + Sealed + Signer + Serialize + Debug,
    T::Response: DeserializeOwned,
{
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

fn try_into_request_signed<T>(
    authentication: &KeySecretAuthentication,
    x: T,
) -> Result<hyper::Request<hyper::Body>, Error>
where
    T: Request + HttpRequest + Sealed + Signer + Serialize + Debug,
    T::Response: DeserializeOwned,
{
    if !<T::Signer as SignerKind>::is_private() {
        if x.method() == nerf::http::Method::GET {
            let params = serde_urlencoded::to_string(&x).map_err(Error::SerializeUrlencodedBody)?;
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
    trace!(uri = uri.to_string(), signed_req = ?signed_req, api_key = authentication.key(), method = method.to_string());
    let mut hmac = HmacSha256::new(authentication.secret().as_bytes().into());
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
            .header("X-MBX-APIKEY", authentication.key.clone())
            .body(hyper::Body::empty())
            .map_err(Error::ConstructHttpRequest)?)
    } else if method == nerf::http::Method::POST || method == nerf::http::Method::DELETE {
        Ok(hyper::Request::builder()
            .uri(full_uri)
            .method(method)
            .header("X-MBX-APIKEY", authentication.key.clone())
            .header("Content-Type", "x-www-form-urlencoded")
            .body(hyper::Body::empty())
            .map_err(Error::ConstructHttpRequest)?)
    } else {
        Err(Error::UnsupportedHttpMethod(method))
    }
}

fn try_from_response<T>(
    x: hyper::Response<hyper::Body>,
) -> Pin<Box<dyn Future<Output = Result<T::Response, Error>>>>
where
    T: Request + HttpRequest + Sealed + Signer + Serialize + Debug,
    T::Response: DeserializeOwned,
{
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
                code: Some(error.code.to_string()),
                msg: Some(error.msg),
            })
        } else {
            let resp = serde_json::from_reader(buf.reader()).map_err(Error::DeserializeJsonBody)?;
            Ok(resp)
        }
    })
}

mod __private {
    use crate::common::Unsupported;

    pub trait Sealed {}
    impl Sealed for Unsupported {}
}
