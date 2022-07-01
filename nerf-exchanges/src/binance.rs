use nerf_macros::get;
use pin_project::pin_project;
use rust_decimal::Decimal;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_with::skip_serializing_none;
use thiserror::Error;

use crate::define_layer;

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
}

pub struct Request<T>(T);

impl<T> nerf::Request for Request<T>
where
    T: nerf::Request,
{
    type Response = Response<T::Response>;
}

impl<T> TryFrom<Request<T>> for hyper::Request<hyper::Body>
where
    T: nerf::Request + nerf::HttpRequest + Serialize,
{
    type Error = Error;

    fn try_from(value: Request<T>) -> Result<Self, Self::Error> {
        let req = value.0;
        if req.method() == nerf::http::Method::GET {
            let params =
                serde_urlencoded::to_string(&req).map_err(Error::SerializeUrlencodedBody)?;
            let uri = req.uri();
            assert!(uri.query().is_none()); // TODO
            Ok(hyper::Request::builder()
                .uri(format!("{uri}?{params}"))
                .method(req.method())
                .body(hyper::Body::empty())
                .map_err(Error::ConstructHttpRequest)?)
        } else {
            let bytes = serde_json::to_vec(&req).map_err(Error::SerializeJsonBody)?;
            Ok(hyper::Request::builder()
                .uri(req.uri())
                .method(req.method())
                .body(bytes.into())
                .map_err(Error::ConstructHttpRequest)?)
        }
    }
}

pub struct Response<T>(T);

impl<T> TryFrom<nerf::Bytes> for Response<T>
where
    T: DeserializeOwned,
{
    type Error = Error;

    fn try_from(value: nerf::Bytes) -> Result<Self, Self::Error> {
        let this = serde_json::from_slice(&value);
        this.map(Response).map_err(Error::DeserializeJsonBody)
    }
}

define_layer!(BinanceLayer, BinanceService, BinanceError, BinanceFuture);

#[skip_serializing_none]
#[derive(Clone, Debug, Serialize)]
#[get("https://api.binance.com/api/v3/trades", response = GetApiV3TradesResponse)]
pub struct GetApiV3Trades {
    pub symbol: String,
    /// Default 500, max 1000
    pub limit: Option<u64>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GetApiV3TradesResponse(Vec<GetApiV3TradesResponseItem>);

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetApiV3TradesResponseItem {
    pub id: i64,
    pub price: Decimal,
    pub qty: Decimal,
    pub quote_qty: Decimal,
    pub time: u64,
    pub is_buyer_maker: bool,
    pub is_best_match: bool,
}
