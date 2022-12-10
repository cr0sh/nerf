use std::{fmt::Debug, future::Future, pin::Pin};

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
    Error, KeySecretAuthentication as Authentication,
};

use super::{BinanceOrderbookItem, OrderType, Side, TimeInForce, __private::Sealed};

fn bool_str<S>(x: &bool, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    if *x {
        "true".serialize(serializer)
    } else {
        "false".serialize(serializer)
    }
}

fn deserialize_bool_str<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: &str = serde::de::Deserialize::deserialize(deserializer)?;

    match s {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(serde::de::Error::unknown_variant(s, &["true", "false"])),
    }
}

fn bool_str_screaming<S>(x: &bool, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    if *x {
        "TRUE".serialize(serializer)
    } else {
        "FALSE".serialize(serializer)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PositionSide {
    Both,
    Long,
    Short,
}

#[derive(Clone, Debug, Serialize)]
#[get("https://fapi.binance.com/fapi/v1/trades", response = GetFapiV1TradesResponse)]
#[tag(Signer = Disabled)]
#[serde(rename_all = "camelCase")]
pub struct GetFapiV1Trades {
    pub symbol: String,
    pub limit: Option<u64>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GetFapiV1TradesResponse(Vec<GetFapiV1TradesResponseItem>);

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetFapiV1TradesResponseItem {
    pub id: u64,
    pub price: Decimal,
    pub qty: Decimal,
    pub quote_qty: Decimal,
    #[serde(with = "ts_milliseconds")]
    pub time: DateTime<Utc>,
    pub is_buyer_maker: bool,
}

#[derive(Clone, Debug, Serialize)]
#[get("https://fapi.binance.com/fapi/v1/depth", response = GetFapiV1DepthResponse)]
#[tag(Signer = Disabled)]
#[serde(rename_all = "camelCase")]
pub struct GetFapiV1Depth {
    pub symbol: String,
    pub limit: Option<u64>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetFapiV1DepthResponse {
    pub last_update_id: u64,
    #[serde(rename = "E", with = "ts_milliseconds")]
    pub message_output_time: DateTime<Utc>,
    #[serde(rename = "T", with = "ts_milliseconds")]
    pub transaction_time: DateTime<Utc>,
    pub bids: Vec<BinanceOrderbookItem>,
    pub asks: Vec<BinanceOrderbookItem>,
}

#[derive(Clone, Debug, Serialize)]
#[get("https://fapi.binance.com/fapi/v2/balance", response = GetFapiV2BalanceResponse)]
#[tag(Signer = Private)]
pub struct GetFapiV2Balance;

#[derive(Clone, Debug, Deserialize)]
pub struct GetFapiV2BalanceResponse(pub Vec<GetFapiV2BalanceResponseItem>);

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetFapiV2BalanceResponseItem {
    pub account_alias: String,
    pub asset: String,
    pub balance: Decimal,
    pub cross_wallet_balance: Decimal,
    pub cross_un_pnl: Decimal,
    pub available_balance: Decimal,
    pub max_withdraw_amount: Decimal,
    pub margin_avaliable: bool,
    #[serde(with = "ts_milliseconds")]
    pub update_time: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize)]
#[get("https://fapi.binance.com/fapi/v2/positionRisk", response = GetFapiV2PositionRiskResponse)]
#[tag(Signer = Private)]
#[skip_serializing_none]
pub struct GetFapiV2PositionRisk {
    pub symbol: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum GetFapiV2PositionRiskResponse {
    Oneway([GetFapiV2PositionRiskResponseOneway; 1]),
    Hedge([GetFapiV2PositionRiskResponseHedge; 2]),
}

impl GetFapiV2PositionRiskResponse {
    /// Returns `None` if the position is in 'hedge mode'.
    pub fn into_oneway(self) -> Option<GetFapiV2PositionRiskResponseOneway> {
        match self {
            Self::Oneway([x]) => Some(x),
            _ => None,
        }
    }

    /// Returns `None` if the position is in 'one-way mode'.
    pub fn into_hedge(
        self,
    ) -> Option<(
        GetFapiV2PositionRiskResponseHedge,
        GetFapiV2PositionRiskResponseHedge,
    )> {
        match self {
            Self::Hedge([x, y]) => Some((x, y)),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetFapiV2PositionRiskResponseOneway {
    pub entry_price: Decimal,
    pub margin_type: String,
    // #[serde(deserialize_with = "deserialize_bool_str")] // FIXME: this causes deserialization failure
    // pub is_auto_add_margin: bool,
    pub isolated_margin: Decimal,
    pub leverage: Decimal,
    pub liquidation_price: Decimal,
    pub mark_price: Decimal,
    pub max_notional_value: Decimal,
    pub position_amt: Decimal,
    pub notional: Decimal,
    pub isolated_wallet: Decimal,
    pub symbol: String,
    #[serde(rename = "unRealizedProfit")]
    pub unrealized_profit: Decimal,
    pub position_side: String,
    #[serde(with = "ts_milliseconds")]
    pub update_time: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetFapiV2PositionRiskResponseHedge {
    pub symbol: String,
    pub position_amt: Decimal,
    pub entry_price: Decimal,
    pub mark_price: Decimal,
    #[serde(rename = "unRealizedProfit")]
    pub unrealized_profit: Decimal,
    pub liquidation_price: Decimal,
    pub leverage: Decimal,
    pub max_notional_value: Decimal,
    pub margin_type: String,
    pub isolated_margin: Decimal,
    #[serde(deserialize_with = "deserialize_bool_str")]
    pub is_auto_add_margin: bool,
    pub position_side: String,
    pub notional: Decimal,
    pub isolated_wallet: Decimal,
    #[serde(with = "ts_milliseconds")]
    pub update_time: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize)]
#[post("https://fapi.binance.com/fapi/v1/order", response = PostFapiV1OrderResponse)]
#[tag(Signer = Private)]
#[serde(rename_all = "camelCase")]
#[skip_serializing_none]
pub struct PostFapiV1Order {
    symbol: String,
    side: Side,
    position_side: Option<PositionSide>,
    #[serde(rename = "type")]
    order_type: OrderType,
    time_in_force: Option<TimeInForce>,
    quantity: Option<Decimal>,
    #[serde(serialize_with = "bool_str")]
    reduce_only: bool,
    price: Option<Decimal>,
    new_client_order_id: Option<String>,
    stop_price: Option<Decimal>,
    close_position: Option<Decimal>,
    activation_price: Option<Decimal>,
    callback_rate: Option<Decimal>,
    working_type: Option<String>,
    #[serde(serialize_with = "bool_str_screaming")]
    price_protect: bool,
    new_order_resp_type: Option<&'static str>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostFapiV1OrderResponse {
    pub client_order_id: String,
    pub cum_qty: Decimal,
    pub cum_quote: Decimal,
    pub executed_qty: Decimal,
    pub order_id: u64,
    pub avg_price: Decimal,
    pub orig_qty: Decimal,
    pub price: Decimal,
    pub reduce_only: bool,
    pub side: Side,
    pub position_side: PositionSide,
    pub status: String,
    pub stop_price: Decimal,
    pub close_position: bool,
    pub symbol: String,
    pub time_in_force: TimeInForce,
    #[serde(rename = "type")]
    pub order_type: OrderType,
    pub orig_type: OrderType,
    pub activate_price: Option<Decimal>,
    pub price_rate: Option<Decimal>,
    #[serde(with = "ts_milliseconds")]
    pub update_time: DateTime<Utc>,
    pub working_type: String,
    pub price_protect: bool,
}

#[derive(Clone, Debug, Serialize)]
#[get("https://fapi.binance.com/fapi/v1/openOrder", response = GetFapiV1OpenOrderResponse)]
#[tag(Signer = Private)]
#[serde(rename_all = "camelCase")]
#[skip_serializing_none]
pub struct GetFapiV1OpenOrder {
    symbol: String,
    order_id: Option<u64>,
    orig_client_order_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetFapiV1OpenOrderResponse {
    pub avg_price: Decimal,
    pub client_order_id: String,
    pub cum_quote: Decimal,
    pub executed_qty: Decimal,
    pub order_id: u64,
    pub orig_qty: Decimal,
    pub orig_type: OrderType,
    pub price: Decimal,
    pub reduce_only: bool,
    pub side: Side,
    pub position_side: PositionSide,
    pub status: String,
    pub stop_price: Decimal,
    pub close_position: bool,
    pub symbol: String,
    #[serde(with = "ts_milliseconds")]
    pub time: DateTime<Utc>,
    pub time_in_force: TimeInForce,
    #[serde(rename = "type")]
    pub order_type: OrderType,
    pub activate_price: Decimal,
    pub price_rate: Option<Decimal>,
    #[serde(with = "ts_milliseconds")]
    pub update_time: DateTime<Utc>,
    pub working_type: String,
    pub price_protect: bool,
}

#[derive(Clone, Debug, Serialize)]
#[get("https://fapi.binance.com/fapi/v1/openOrders", response = GetFapiV1OpenOrdersResponse)]
#[tag(Signer = Private)]
#[serde(rename_all = "camelCase")]
#[skip_serializing_none]
pub struct GetFapiV1OpenOrders {
    symbol: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GetFapiV1OpenOrdersResponse(Vec<GetFapiV1OpenOrderResponse>);

#[derive(Clone, Debug, Serialize)]
#[delete("https://fapi.binance.com/fapi/v1/order", response = DeleteFapiV1OrderResponse)]
#[tag(Signer = Private)]
#[serde(rename_all = "camelCase")]
#[skip_serializing_none]
pub struct DeleteFapiV1Order {
    pub symbol: String,
    pub order_id: Option<u64>,
    pub orig_client_order_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteFapiV1OrderResponse {
    pub client_order_id: String,
    pub cum_qty: Decimal,
    pub cum_quote: Decimal,
    pub executed_qty: Decimal,
    pub order_id: u64,
    pub orig_qty: Decimal,
    pub orig_type: OrderType,
    pub price: Decimal,
    pub reduce_only: bool,
    pub side: Side,
    pub position_side: PositionSide,
    pub status: String,
    pub stop_price: Decimal,
    pub close_position: bool,
    pub symbol: String,
    pub time_in_force: String,
    #[serde(rename = "type")]
    pub order_type: OrderType,
    pub activate_price: Option<Decimal>,
    pub price_rate: Option<Decimal>,
    #[serde(with = "ts_milliseconds")]
    pub update_time: DateTime<Utc>,
    pub working_type: String,
    pub price_protect: bool,
}

#[derive(Clone, Debug, Serialize)]
#[delete("https://fapi.binance.com/fapi/v1/allOpenOrders", response = DeleteFapiV1AllOpenOrdersResponse)]
#[tag(Signer = Private)]
#[serde(rename_all = "camelCase")]
#[skip_serializing_none]
pub struct DeleteFapiV1AllOpenOrders {
    pub symbol: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DeleteFapiV1AllOpenOrdersResponse {
    pub code: i64,
    pub msg: String,
}

impl From<common::GetTrades> for GetFapiV1Trades {
    fn from(x: common::GetTrades) -> Self {
        Self {
            symbol: format!("{}{}", x.market.base(), x.market.quote()),
            limit: None,
        }
    }
}

impl From<common::GetOrderbook> for GetFapiV1Depth {
    fn from(x: common::GetOrderbook) -> Self {
        Self {
            symbol: format!("{}{}", x.market.base(), x.market.quote()),
            limit: x.ticks,
        }
    }
}

impl From<common::GetBalance> for GetFapiV2Balance {
    fn from(_: common::GetBalance) -> Self {
        Self
    }
}

impl From<common::GetPosition> for GetFapiV2PositionRisk {
    fn from(x: common::GetPosition) -> Self {
        assert_eq!(*x.market.kind(), common::MarketKind::UsdMarginedPerpetual);
        Self {
            symbol: Some(format!("{}{}", x.market.base(), x.market.quote())),
        }
    }
}

impl From<common::PlaceOrder> for PostFapiV1Order {
    fn from(x: common::PlaceOrder) -> Self {
        Self {
            symbol: format!("{}{}", x.market.base(), x.market.quote()),
            side: match x.order.side() {
                common::Side::Buy => Side::Buy,
                common::Side::Sell => Side::Sell,
            },
            position_side: Some(PositionSide::Both), // TODO: can `common::PlaceOrder` support two-way mode?
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
            reduce_only: false, // TODO
            price: x.order.price(),
            new_client_order_id: None,
            stop_price: x.order.stop_price(),
            close_position: None,
            activation_price: None,
            callback_rate: None,
            working_type: None,
            price_protect: false,
            new_order_resp_type: Some("FULL"),
        }
    }
}

impl From<common::GetOrders> for GetFapiV1OpenOrders {
    fn from(x: common::GetOrders) -> Self {
        Self {
            symbol: Some(format!("{}{}", x.market.base(), x.market.quote())),
        }
    }
}

impl From<common::GetAllOrders> for GetFapiV1OpenOrders {
    fn from(_: common::GetAllOrders) -> Self {
        Self { symbol: None }
    }
}

impl From<common::CancelOrder> for DeleteFapiV1Order {
    fn from(x: common::CancelOrder) -> Self {
        Self {
            symbol: format!("{}{}", x.market.base(), x.market.quote()),
            order_id: Some(x.order_id.parse().expect("cannot parse order_id")),
            orig_client_order_id: None,
        }
    }
}

impl IntoCommon for GetFapiV1DepthResponse {
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
        )
    }
}

#[derive(Clone, Debug)]
pub struct BinanceFuturesClient<S>(S);

impl<S> BinanceFuturesClient<S> {
    pub fn new(x: S) -> Self {
        Self(x)
    }

    pub fn with_auth(self, authentication: Authentication) -> BinanceFuturesPrivateClient<S> {
        BinanceFuturesPrivateClient {
            client: self,
            authentication,
        }
    }
}

#[derive(Clone, Debug)]
pub struct BinanceFuturesPrivateClient<S> {
    client: BinanceFuturesClient<S>,
    authentication: Authentication,
}

impl<T, S> Client<T> for BinanceFuturesClient<S>
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
        super::try_into_request(x)
    }

    fn try_from_response(x: hyper::Response<hyper::Body>) -> Self::TryFromResponseFuture {
        super::try_from_response::<T>(x)
    }
}

impl<T, S> Client<T> for BinanceFuturesPrivateClient<S>
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
        super::try_into_request_signed(&self.authentication, x)
    }

    fn try_from_response(x: hyper::Response<hyper::Body>) -> Self::TryFromResponseFuture {
        super::try_from_response::<T>(x)
    }
}

impl<S> CommonOps for BinanceFuturesClient<S> {
    type GetTickersRequest = Unsupported;

    type GetTradesRequest = GetFapiV1Trades;

    type GetOrderbookRequest = GetFapiV1Depth;

    type GetOrdersRequest = Unsupported;

    type GetAllOrdersRequest = Unsupported;

    type PlaceOrderRequest = Unsupported;

    type CancelOrderRequest = Unsupported;

    type CancelAllOrdersRequest = Unsupported;

    type GetBalanceRequest = Unsupported;

    type GetPositionRequest = Unsupported;
}

impl<S> tower::Service<Unsupported> for BinanceFuturesClient<S> {
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

impl<S> CommonOps for BinanceFuturesPrivateClient<S> {
    type GetTickersRequest = Unsupported;

    type GetTradesRequest = GetFapiV1Trades;

    type GetOrderbookRequest = GetFapiV1Depth;

    type GetOrdersRequest = GetFapiV1OpenOrders;

    type GetAllOrdersRequest = GetFapiV1OpenOrders;

    type PlaceOrderRequest = PostFapiV1Order;

    type CancelOrderRequest = DeleteFapiV1Order;

    type CancelAllOrdersRequest = Unsupported;

    type GetBalanceRequest = Unsupported;

    type GetPositionRequest = GetFapiV2PositionRisk;
}

impl<S> tower::Service<Unsupported> for BinanceFuturesPrivateClient<S> {
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
