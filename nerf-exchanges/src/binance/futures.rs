use chrono::serde::ts_milliseconds;
use chrono::{DateTime, Utc};
use nerf::{get, tag};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::__private::Sealed;
use super::{Disabled, Signer};

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
