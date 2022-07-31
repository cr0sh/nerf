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
use nerf_macros::{get, post, tag};
use rust_decimal::Decimal;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_with::skip_serializing_none;
use sha2::Sha256;
use thiserror::Error;
use tracing::trace;

use crate::common::{self, CommonOps, Unsupported};

use self::__private::Sealed;

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
}

impl From<Infallible> for Error {
    fn from(x: Infallible) -> Self {
        match x {}
    }
}

#[derive(Clone)]
pub struct Authentication {
    key: String,
    secret: String,
}

impl Authentication {
    pub fn new(key: String, secret: String) -> Self {
        Self { key, secret }
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
}

#[skip_serializing_none]
#[derive(Clone, Debug, Serialize)]
#[get("https://api.binance.com/api/v3/trades", response = GetApiV3TradesResponse)]
#[tag(Signer = Disabled)]
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
    #[serde(with = "ts_milliseconds")]
    pub time: DateTime<Utc>,
    pub is_buyer_maker: bool,
    pub is_best_match: bool,
}

#[skip_serializing_none]
#[derive(Clone, Debug, Serialize)]
#[get("https://api.binance.com/api/v3/depth", response = GetApiV3DepthResponse)]
#[tag(Signer = Disabled)]
pub struct GetApiV3Depth {
    pub symbol: String,
    pub limit: Option<u64>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetApiV3DepthResponse {
    pub last_update_id: u64,
    pub bids: Vec<BinanceOrderbookItem>,
    pub asks: Vec<BinanceOrderbookItem>,
}

#[derive(Clone, Debug)]
pub struct BinanceOrderbookItem {
    pub price: Decimal,
    pub quantity: Decimal,
}

impl<'de> Deserialize<'de> for BinanceOrderbookItem {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let inner: (Decimal, Decimal) = Deserialize::deserialize(deserializer)?;
        Ok(Self {
            price: inner.0,
            quantity: inner.1,
        })
    }
}

#[skip_serializing_none]
#[derive(Clone, Debug, Serialize)]
#[get("https://api.binance.com/api/v3/account", response = GetApiV3AccountResponse)]
#[tag(Signer = UserDataSigner)]
pub struct GetApiV3Account {}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetApiV3AccountResponse {
    pub maker_commission: Decimal,
    pub taker_commission: Decimal,
    pub buyer_commission: Decimal,
    pub seller_commission: Decimal,
    pub can_trade: bool,
    pub can_withdraw: bool,
    pub can_deposit: bool,
    #[serde(with = "ts_milliseconds")]
    pub update_time: DateTime<Utc>,
    #[serde(skip)]
    pub account_type: (),
    pub balances: Vec<GetApiV3AccountBalanceItem>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetApiV3AccountBalanceItem {
    pub asset: String,
    pub free: Decimal,
    pub locked: Decimal,
}

#[skip_serializing_none]
#[derive(Clone, Debug, Serialize)]
#[post("https://api.binance.com/api/v3/order", response = PostApiV3OrderResponse)]
#[tag(Signer = UserDataSigner)]
#[serde(rename_all = "camelCase")]
pub struct PostApiV3Order {
    pub symbol: String,
    pub side: Side,
    #[serde(rename = "type")]
    pub order_type: OrderType,
    pub time_in_force: Option<TimeInForce>,
    pub quantity: Option<Decimal>,
    pub quote_order_qty: Option<Decimal>,
    pub price: Option<Decimal>,
    pub new_client_order_id: Option<String>,
    pub stop_price: Option<Decimal>,
    pub trailing_delta: Option<u64>,
    pub iceberg_qty: Option<Decimal>,
    pub new_order_resp_type: Option<&'static str>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostApiV3OrderResponse {
    pub symbol: String,
    pub order_id: u64,
    pub order_list_id: i64,
    pub client_order_id: Option<String>,
    #[serde(with = "ts_milliseconds")]
    pub transact_time: DateTime<Utc>, // TODO: better deserializatoin
    pub price: Option<Decimal>,
    pub orig_qty: Option<Decimal>,
    pub executed_qty: Option<Decimal>,
    pub cumulative_quote_qty: Option<Decimal>,
    pub status: Option<String>, // TODO
    pub time_in_force: Option<TimeInForce>,
    #[serde(rename = "type")]
    pub order_type: Option<OrderType>,
    pub side: Option<Side>,
}

#[derive(Clone, Debug, Serialize)]
#[post("https://api.binance.com/api/v3/openOrders", response = GetApiV3OpenOrdersResponse)]
#[tag(Signer = UserDataSigner)]
#[serde(rename_all = "camelCase")]
pub struct GetApiV3OpenOrders {
    pub symbol: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(transparent)]
pub struct GetApiV3OpenOrdersResponse(Vec<GetApiV3OpenOrdersResponseItem>);

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetApiV3OpenOrdersResponseItem {
    pub symbol: String,
    pub order_id: u64,
    pub order_list_id: i64,
    pub client_order_id: String,
    pub price: Decimal,
    pub orig_qty: Decimal,
    pub executed_qty: Decimal,
    pub cumulative_quote_qty: Decimal,
    pub status: String, // TODO: make this enum
    pub time_in_force: TimeInForce,
    #[serde(rename = "type")]
    pub order_type: OrderType,
    pub side: Side,
    pub stop_price: Decimal,
    pub iceberg_qty: Decimal,
    #[serde(with = "ts_milliseconds")]
    pub time: DateTime<Utc>,
    #[serde(with = "ts_milliseconds")]
    pub update_time: DateTime<Utc>,
    pub is_working: bool,
    pub orig_quote_order_qty: Decimal,
}

impl From<common::GetTrades> for GetApiV3Trades {
    fn from(x: common::GetTrades) -> Self {
        GetApiV3Trades {
            symbol: format!("{}{}", x.market.base(), x.market.quote()),
            limit: None,
        }
    }
}

impl From<common::GetOrderbook> for GetApiV3Depth {
    fn from(x: common::GetOrderbook) -> Self {
        GetApiV3Depth {
            symbol: format!("{}{}", x.market.base(), x.market.quote()),
            limit: x.ticks,
        }
    }
}

impl From<common::GetOrders> for GetApiV3OpenOrders {
    fn from(x: common::GetOrders) -> Self {
        GetApiV3OpenOrders {
            symbol: format!("{}{}", x.market.base(), x.market.quote()),
        }
    }
}

impl From<common::PlaceOrder> for PostApiV3Order {
    fn from(x: common::PlaceOrder) -> Self {
        PostApiV3Order {
            symbol: format!("{}{}", x.market.base(), x.market.quote()),
            side: match x.order.side() {
                common::Side::Buy => Side::Buy,
                common::Side::Sell => Side::Sell,
            },
            order_type: match x.order {
                common::Order::Market { .. } => OrderType::Market,
                common::Order::Limit { .. } => OrderType::Limit,
                common::Order::StopMarket { .. } => todo!(), // FIXME
                common::Order::StopLimit { .. } => todo!(),  // FIXME
            },
            time_in_force: x.order.time_in_force().map(|tif| match tif {
                common::TimeInForce::GoodTilCancled => TimeInForce::GoodTilCanceled,
                common::TimeInForce::ImmediateOrCancel => TimeInForce::ImmediateOrCancel,
                common::TimeInForce::FillOrKill => TimeInForce::FillOrKill,
            }),
            quantity: Some(x.order.quantity()),
            quote_order_qty: None,
            price: x.order.price(),
            new_client_order_id: None,
            stop_price: x.order.stop_price(),
            trailing_delta: None,
            iceberg_qty: None,
            new_order_resp_type: Some("FULL"),
        }
    }
}

impl From<common::GetBalance> for GetApiV3Account {
    fn from(_: common::GetBalance) -> Self {
        GetApiV3Account {} // FIXME: GetBalance.asset is ignored
    }
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
        const SIGN_RECV_WINDOW_MILLIS: u64 = 2000;

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
        trace!(uri = uri.to_string(), signed_req = ?signed_req, api_key = self.authentication.key, method = method.to_string());
        let mut hmac = HmacSha256::new(self.authentication.secret.as_bytes().into());
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

        if method == nerf::http::Method::GET {
            assert!(uri.query().is_none()); // TODO
            let full_uri = format!("{uri}?{params}{signature}");
            trace!(full_uri = full_uri, "Method is GET");
            Ok(hyper::Request::builder()
                .uri(full_uri)
                .method(method)
                .header("X-MBX-APIKEY", self.authentication.key.clone())
                .body(hyper::Body::empty())
                .map_err(Error::ConstructHttpRequest)?)
        } else if method == nerf::http::Method::POST {
            let body = format!("{params}{signature}");
            trace!(body = body, "Method is POST");
            Ok(hyper::Request::builder()
                .uri(uri)
                .method(method)
                .header("X-MBX-APIKEY", self.authentication.key.clone())
                .header("Content-Type", "x-www-form-urlencoded")
                .body(hyper::Body::from(body))
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
    type GetTradesRequest = GetApiV3Trades;

    type GetOrderbookRequest = GetApiV3Depth;

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
    type GetTradesRequest = GetApiV3Trades;

    type GetOrderbookRequest = GetApiV3Depth;

    type GetOrdersRequest = GetApiV3OpenOrders;

    type GetAllOrdersRequest = Unsupported;

    type PlaceOrderRequest = PostApiV3Order;

    type CancelOrderRequest = Unsupported;

    type CancelAllOrdersRequest = Unsupported;

    type GetBalanceRequest = GetApiV3Account;

    type GetPositionRequest = Unsupported;
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

    pub trait Signer {}
}
