use std::{
    fmt::{Debug, Write},
    future::Future,
    pin::Pin,
};

use chrono::{serde::ts_milliseconds, DateTime, Utc};
use hmac::{Hmac, Mac};
use hyper::body::Buf;
use nerf::{define_layer, http::StatusCode, Signer, TryIntoResponse};
use nerf_macros::{get, post};
use rust_decimal::Decimal;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_with::skip_serializing_none;
use sha2::Sha256;
use thiserror::Error;
use tracing::trace;

use crate::common::{self, CommonOps, Unsupported};

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

impl<T> TryFrom<BinanceSignerWrapped<Request<T>>> for hyper::Request<hyper::Body>
where
    T: nerf::Request + nerf::HttpRequest + Serialize + Debug,
{
    type Error = Error;

    fn try_from(value: BinanceSignerWrapped<Request<T>>) -> Result<Self, Self::Error> {
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

        let req = (value.0).0;
        let method = req.method();
        let uri = req.uri();
        let signed_req = SignedRequest {
            req,
            recv_window: SIGN_RECV_WINDOW_MILLIS,
            timestamp: chrono::Utc::now(),
        };
        trace!(uri = uri.to_string(), signed_req = ?signed_req, api_key = (value.1).key, method = method.to_string());
        let mut hmac = HmacSha256::new((value.1).secret.as_bytes().into());
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
                .header("X-MBX-APIKEY", (value.1).key)
                .body(hyper::Body::empty())
                .map_err(Error::ConstructHttpRequest)?)
        } else if method == nerf::http::Method::POST {
            let body = format!("{params}{signature}");
            trace!(body = body, "Method is POST");
            Ok(hyper::Request::builder()
                .uri(uri)
                .method(method)
                .header("X-MBX-APIKEY", (value.1).key)
                .header("Content-Type", "x-www-form-urlencoded")
                .body(hyper::Body::from(body))
                .map_err(Error::ConstructHttpRequest)?)
        } else {
            Err(Error::UnsupportedHttpMethod(method))
        }
    }
}

impl<T> TryIntoResponse<Response<T>> for hyper::Response<hyper::Body>
where
    T: DeserializeOwned,
{
    type Error = Error;

    type Future = Pin<Box<dyn Future<Output = Result<Response<T>, Error>>>>;

    fn try_into_response(self) -> Self::Future {
        Box::pin(async move {
            let status = self.status();
            let buf = hyper::body::aggregate(self).await?;
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
                let resp: T =
                    serde_json::from_reader(buf.reader()).map_err(Error::DeserializeJsonBody)?;
                Ok(Response(resp))
            }
        })
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
    #[serde(with = "ts_milliseconds")]
    pub time: DateTime<Utc>,
    pub is_buyer_maker: bool,
    pub is_best_match: bool,
}

#[skip_serializing_none]
#[derive(Clone, Debug, Serialize)]
#[get("https://api.binance.com/api/v3/depth", response = GetApiV3DepthResponse)]
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
#[get("https://api.binance.com/api/v3/account", response = GetApiV3AccountResponse, signer = BinanceSigner)]
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
#[post("https://api.binance.com/api/v3/order", response = PostApiV3OrderResponse, signer = BinanceSigner)]
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
#[post("https://api.binance.com/api/v3/openOrders", response = GetApiV3OpenOrdersResponse, signer = BinanceSigner)]
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

pub struct BinanceSigner(());

pub struct BinanceSignerWrapped<R>(R, Authentication);

impl<R> nerf::Request for BinanceSignerWrapped<R>
where
    R: nerf::Request,
{
    type Response = R::Response;
}

impl<R> Signer<R> for BinanceSigner {
    type Wrapped = BinanceSignerWrapped<R>;
    type Context = Authentication;

    fn wrap_signer(req: R, context: Self::Context) -> Self::Wrapped {
        BinanceSignerWrapped(req, context)
    }
}

impl<R> Signer<Request<R>> for () {
    type Wrapped = Request<R>;
    type Context = Authentication;
    fn wrap_signer(req: Request<R>, _context: Self::Context) -> Self::Wrapped {
        req
    }
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

impl<S> CommonOps for BinanceService<S> {
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

impl<S> tower::Service<Unsupported> for BinanceService<S> {
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
