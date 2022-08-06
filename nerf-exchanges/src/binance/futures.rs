use chrono::serde::ts_milliseconds;
use chrono::{DateTime, Utc};
use nerf::{delete, get, post, tag};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::common::{
    self, CancelOrder, GetAllOrders, GetBalance, GetOrderbook, GetOrders, GetPosition, GetTrades,
    PlaceOrder, MarketKind,
};

use super::__private::Sealed;
use super::{BinanceOrderbookItem, Disabled, OrderType, Side, Signer, TimeInForce, UserDataSigner};

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
#[tag(Signer = UserDataSigner)]
pub struct GetFapiV2Balance;

#[derive(Clone, Debug, Deserialize)]
pub struct GetFapiV2BalanceResponse(Vec<GetFapiV2BalanceResponseItem>);

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
#[get("https://fapi.binance.com/fapi/v2/balance", response = GetFapiV2PositionRiskResponse)]
#[tag(Signer = UserDataSigner)]
#[skip_serializing_none]
pub struct GetFapiV2PositionRisk {
    symbol: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum GetFapiV2PositionRiskResponse {
    Oneway([GetFapiV2PositionRiskResponseOneway; 1]),
    Hedge([GetFapiV2PositionRiskResponseHedge; 2]),
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetFapiV2PositionRiskResponseOneway {
    pub entry_price: Decimal,
    pub margin_type: String,
    pub is_auto_add_margin: bool,
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
    pub is_auto_add_margin: Decimal,
    pub position_side: String,
    pub notional: Decimal,
    pub isolated_wallet: Decimal,
    #[serde(with = "ts_milliseconds")]
    pub update_time: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize)]
#[post("https://fapi.binance.com/fapi/v1/order", response = PostFapiV1OrderResponse)]
#[tag(Signer = UserDataSigner)]
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
#[tag(Signer = UserDataSigner)]
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
#[tag(Signer = UserDataSigner)]
#[serde(rename_all = "camelCase")]
#[skip_serializing_none]
pub struct GetFapiV1OpenOrders {
    symbol: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GetFapiV1OpenOrdersResponse(Vec<GetFapiV1OpenOrderResponse>);

#[derive(Clone, Debug, Serialize)]
#[delete("https://fapi.binance.com/fapi/v1/order", response = DeleteFapiV1OrderResponse)]
#[tag(Signer = UserDataSigner)]
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
    pub activate_price: Decimal,
    pub price_rate: Decimal,
    #[serde(with = "ts_milliseconds")]
    pub update_time: DateTime<Utc>,
    pub working_type: String,
    pub price_protect: bool,
}

#[derive(Clone, Debug, Serialize)]
#[delete("https://fapi.binance.com/fapi/v1/allOpenOrders", response = DeleteFapiV1AllOpenOrdersResponse)]
#[tag(Signer = UserDataSigner)]
#[serde(rename_all = "camelCase")]
#[skip_serializing_none]
pub struct DeleteFapiV1AllOpenOrders {
    pub symbol: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DeleteFapiV1AllOpenOrdersResponse {
    pub code: String,
    pub msg: String,
}

impl From<GetTrades> for GetFapiV1Trades {
    fn from(x: GetTrades) -> Self {
        Self {
            symbol: format!("{}{}", x.market.base(), x.market.quote()),
            limit: None,
        }
    }
}

impl From<GetOrderbook> for GetFapiV1Depth {
    fn from(x: GetOrderbook) -> Self {
        Self {
            symbol: format!("{}{}", x.market.base(), x.market.quote()),
            limit: x.ticks,
        }
    }
}

impl From<GetBalance> for GetFapiV2Balance {
    fn from(_: GetBalance) -> Self {
        Self
    }
}

impl From<GetPosition> for GetFapiV2PositionRisk {
    fn from(x: GetPosition) -> Self {
        assert_eq!(*x.market.kind(), MarketKind::UsdMarginedPerpetual);
        Self {
            symbol: Some(format!("{}{}", x.market.base(), x.market.quote())),
        }
    }
}

impl From<PlaceOrder> for PostFapiV1Order {
    fn from(x: PlaceOrder) -> Self {
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

impl From<GetOrders> for GetFapiV1OpenOrders {
    fn from(x: GetOrders) -> Self {
        Self {
            symbol: Some(format!("{}{}", x.market.base(), x.market.quote())),
        }
    }
}

impl From<GetAllOrders> for GetFapiV1OpenOrders {
    fn from(_: GetAllOrders) -> Self {
        Self { symbol: None }
    }
}

impl From<CancelOrder> for DeleteFapiV1Order {
    fn from(x: CancelOrder) -> Self {
        Self {
            symbol: format!("{}{}", x.market.base(), x.market.quote()),
            order_id: Some(x.order_id.parse().expect("cannot parse order_id")),
            orig_client_order_id: None,
        }
    }
}
