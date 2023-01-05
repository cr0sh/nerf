use std::{collections::HashMap, fmt::Debug, future::Future, pin::Pin};

use chrono::{serde::ts_milliseconds, DateTime, Utc};
use nerf::{delete, get, post, tag, Client, HttpRequest, Request};
use rust_decimal::Decimal;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::{
    common::{
        self, CommonOps, Disabled, IntoCommon, Orderbook, OrderbookItem, Private, Signer,
        Unsupported,
    },
    KeySecretAuthentication as Authentication,
};

use super::{Error, OrderType, Side, TimeInForce, __private::Sealed};

#[skip_serializing_none]
#[derive(Clone, Debug, Serialize)]
#[get("https://api.binance.com/api/v3/ticker/bookTicker", response = GetApiV3BookTickerResponse)]
#[tag(Signer = Disabled)]
pub struct GetApiV3BookTicker {
    pub symbols: Option<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GetApiV3BookTickerResponse(pub Vec<GetApiV3BookTickerResponseItem>);

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetApiV3BookTickerResponseItem {
    pub symbol: String,
    pub bid_price: Decimal,
    pub bid_qty: Decimal,
    pub ask_price: Decimal,
    pub ask_qty: Decimal,
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
#[tag(Signer = Private)]
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
#[tag(Signer = Private)]
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
    #[serde(with = "ts_milliseconds")]
    pub update_time: DateTime<Utc>,
    pub working_type: String,
    pub price_protect: bool,
}

#[derive(Clone, Debug, Serialize)]
#[post("https://api.binance.com/api/v3/openOrders", response = GetApiV3OpenOrdersResponse)]
#[tag(Signer = Private)]
#[serde(rename_all = "camelCase")]
pub struct GetApiV3OpenOrders {
    pub symbol: Option<String>,
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

#[derive(Clone, Debug, Serialize)]
#[delete("https://api.binance.com/api/v3/order", response = DeleteApiV3OrdersResponse)]
#[tag(Signer = Private)]
#[serde(rename_all = "camelCase")]
pub struct DeleteApiV3Orders {
    pub symbol: String,
    pub order_id: Option<u64>,
    pub orig_client_order_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteApiV3OrdersResponse {
    pub symbol: String,
    pub order_id: u64,
    pub orig_client_order_id: String,
    pub status: String,
    // TODO: Implement other fields
}

impl From<common::GetTickers> for GetApiV3BookTicker {
    fn from(_: common::GetTickers) -> Self {
        GetApiV3BookTicker { symbols: None }
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
            symbol: Some(format!("{}{}", x.market.base(), x.market.quote())),
        }
    }
}

impl From<common::GetAllOrders> for GetApiV3OpenOrders {
    fn from(_: common::GetAllOrders) -> Self {
        GetApiV3OpenOrders { symbol: None }
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
                common::TimeInForce::GoodTilCrossing => TimeInForce::GoodTilCrossing,
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
        GetApiV3Account {}
    }
}

impl From<common::CancelOrder> for DeleteApiV3Orders {
    fn from(x: common::CancelOrder) -> Self {
        Self {
            symbol: format!("{}{}", x.market.base(), x.market.quote()),
            order_id: Some(x.order_id.parse().expect("Cannot parse order_id")),
            orig_client_order_id: None,
        }
    }
}

impl IntoCommon for GetApiV3BookTickerResponse {
    type Output = HashMap<common::Market, common::Ticker>;

    fn into_common(self) -> Self::Output {
        fn split_end<'a>(symbol: &'a str, end: &'static str) -> Option<(&'a str, &'a str)> {
            symbol
                .strip_suffix(end)
                .map(|x| (x, symbol.strip_prefix(x).unwrap()))
        }

        self.0
            .into_iter()
            .filter_map(|x| {
                let (base, quote) = split_end(&x.symbol, "BTC")
                    .or_else(|| split_end(&x.symbol, "USDT"))
                    .or_else(|| split_end(&x.symbol, "BUSD"))?;
                Some((
                    common::Market::from(format!("spot:{base}/{quote}")),
                    common::Ticker::new(x.bid_price, x.ask_price),
                ))
            })
            .collect()
    }
}

impl IntoCommon for GetApiV3DepthResponse {
    type Output = Orderbook;

    fn into_common(self) -> Self::Output {
        Orderbook::new(
            self.bids
                .iter()
                .map(|&BinanceOrderbookItem { price, quantity }| OrderbookItem { price, quantity })
                .collect(),
            self.asks
                .iter()
                .map(|&BinanceOrderbookItem { price, quantity }| OrderbookItem { price, quantity })
                .collect(),
            None,
        )
    }
}

#[derive(Clone, Debug)]
pub struct BinanceSpotClient<S>(S);

impl<S> BinanceSpotClient<S> {
    pub fn new(x: S) -> Self {
        Self(x)
    }

    pub fn with_auth(self, authentication: Authentication) -> BinanceSpotPrivateClient<S> {
        BinanceSpotPrivateClient {
            client: self,
            authentication,
        }
    }
}

#[derive(Clone, Debug)]
pub struct BinanceSpotPrivateClient<S> {
    client: BinanceSpotClient<S>,
    authentication: Authentication,
}

impl<T, S> Client<T> for BinanceSpotClient<S>
where
    T: Request + HttpRequest + Sealed + Signer<Signer = Disabled> + Serialize + Debug,
    T::Response: DeserializeOwned,
{
    type Service = S;

    type Error = Error;

    type TryFromResponseFuture =
        Pin<Box<dyn Future<Output = Result<T::Response, Self::Error>> + Send + Sync + 'static>>;

    fn service(&mut self) -> &mut Self::Service {
        &mut self.0
    }

    fn try_into_request(&mut self, x: T) -> Result<hyper::Request<hyper::Body>, Self::Error> {
        super::try_into_request(x)
    }

    fn try_from_response(x: hyper::Response<hyper::Body>) -> Self::TryFromResponseFuture {
        super::try_from_response::<T>(x)
    }
}

impl<T, S> Client<T> for BinanceSpotPrivateClient<S>
where
    T: Request + HttpRequest + Sealed + Signer + Serialize + Debug,
    T::Response: DeserializeOwned,
{
    type Service = S;

    type Error = Error;

    type TryFromResponseFuture =
        Pin<Box<dyn Future<Output = Result<T::Response, Self::Error>> + Send + Sync + 'static>>;

    fn service(&mut self) -> &mut Self::Service {
        &mut self.client.0
    }

    fn try_into_request(&mut self, x: T) -> Result<hyper::Request<hyper::Body>, Self::Error> {
        super::try_into_request_signed(&self.authentication, x)
    }

    fn try_from_response(x: hyper::Response<hyper::Body>) -> Self::TryFromResponseFuture {
        super::try_from_response::<T>(x)
    }
}

impl<S> CommonOps for BinanceSpotClient<S> {
    type GetTickersRequest = GetApiV3BookTicker;

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

impl<S> tower::Service<Unsupported> for BinanceSpotClient<S> {
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

impl<S> CommonOps for BinanceSpotPrivateClient<S> {
    type GetTickersRequest = GetApiV3BookTicker;

    type GetTradesRequest = GetApiV3Trades;

    type GetOrderbookRequest = GetApiV3Depth;

    type GetOrdersRequest = GetApiV3OpenOrders;

    type GetAllOrdersRequest = Unsupported; // FIXME: TriExchange requires ExtractMarketKind for a common request type

    type PlaceOrderRequest = PostApiV3Order;

    type CancelOrderRequest = DeleteApiV3Orders;

    type CancelAllOrdersRequest = Unsupported;

    type GetBalanceRequest = Unsupported; // FIXME: TriExchange requires ExtractMarketKind for a common request type

    type GetPositionRequest = Unsupported;
}

impl<S> tower::Service<Unsupported> for BinanceSpotPrivateClient<S> {
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
