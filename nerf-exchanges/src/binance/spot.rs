use chrono::{serde::ts_milliseconds, DateTime, Utc};
use nerf::{delete, get, post, tag};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::common::{self, GetTrades, IntoCommon, Orderbook, OrderbookItem};

use super::{Disabled, OrderType, Side, Signer, TimeInForce, UserDataSigner, __private::Sealed};

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
    #[serde(with = "ts_milliseconds")]
    pub update_time: DateTime<Utc>,
    pub working_type: String,
    pub price_protect: bool,
}

#[derive(Clone, Debug, Serialize)]
#[post("https://api.binance.com/api/v3/openOrders", response = GetApiV3OpenOrdersResponse)]
#[tag(Signer = UserDataSigner)]
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
#[tag(Signer = UserDataSigner)]
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

impl From<GetTrades> for GetApiV3Trades {
    fn from(x: GetTrades) -> Self {
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

impl IntoCommon<Orderbook> for GetApiV3DepthResponse {
    fn into_common(self) -> Orderbook {
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
