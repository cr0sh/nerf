//! Common types across various exchanges.

use std::str::FromStr;

use rust_decimal::Decimal;
use thiserror::Error;

pub type Asset = String;

#[derive(Debug, PartialEq, Eq)]
pub struct Market {
    /// Base asset
    base: Asset,
    /// Quote asset
    quote: Asset,
    /// Market kind
    kind: MarketKind,
}

impl Market {
    pub fn new(base: String, quote: String, kind: MarketKind) -> Self {
        Self { base, quote, kind }
    }

    pub fn base(&self) -> &str {
        self.base.as_ref()
    }

    pub fn quote(&self) -> &str {
        self.quote.as_ref()
    }

    pub fn kind(&self) -> &MarketKind {
        &self.kind
    }
}

#[derive(Error, Debug)]
pub enum MarketParseError {
    #[error("cannot parse market {0}")]
    Failure(String),
    #[error("invalid kind {0}")]
    InvalidKind(String),
}

impl FromStr for Market {
    type Err = MarketParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (kind, pair) = s
            .split_once(':')
            .ok_or_else(|| MarketParseError::Failure(s.to_string()))?;
        let (base, quote) = pair
            .split_once('/')
            .ok_or_else(|| MarketParseError::Failure(s.to_string()))?;
        Ok(Market::new(
            base.to_string(),
            quote.to_string(),
            kind.parse()?,
        ))
    }
}

#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum MarketKind {
    /// Spot market
    Spot,
    /// USD(or USD stablecoin)-margined perpetual futures contract market
    UsdMarginedPerpetual,
    /// Coin-margined(a.k.a inverse) futures perpetual contract market
    CoinMarginedPerpetual,
    // /// USD(or USD stablecoin)-margined quarterly futures contract market
    // ///
    // /// TODO: decide how to specify a quarter
    // UseMarginedQuarterly,
    // /// Coin-margined(a.k.a inverse) futures quarterly contract market
    // ///
    // /// TODO: decide how to specify a quarter
    // CoinMarginedQuaterly,
}

impl FromStr for MarketKind {
    type Err = MarketParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "spot" => Ok(MarketKind::Spot),
            "perp" => Ok(MarketKind::UsdMarginedPerpetual),
            "inverse" => Ok(MarketKind::CoinMarginedPerpetual),
            other => Err(MarketParseError::InvalidKind(other.to_string())),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Order {
    Market {
        side: Side,
        size: Decimal,
    },
    Limit {
        side: Side,
        size: Decimal,
        price: Decimal,
    },
    StopMarket {
        side: Side,
        stop_price: Decimal,
        size: Decimal,
    },
    StopLimit {
        side: Side,
        stop_price: Decimal,
        size: Decimal,
        price: Decimal,
    },
}

#[derive(Debug)]
pub struct GetTrades {
    pub market: Market,
}

#[derive(Debug)]
pub struct GetOrderbook {
    pub market: Market,
    /// 'Desired' ticks to fetch. Tick is counted on one side, so returned orderbook
    /// may have `2*ticks` entries.
    /// Note that this is not a strict requirement, so callers should expect or rely
    /// that the returned orderbook to have exactly `2*ticks` entries.
    pub ticks: u64,
}

#[derive(Debug)]
pub struct GetOrders {
    pub market: Market,
}

#[derive(Debug)]
pub struct GetAllOrders;

#[derive(Debug)]
pub struct PlaceOrder {
    pub market: Market,
    pub order: Order,
}

#[derive(Debug)]
pub struct CancelOrder {
    pub order_id: String, // TODO: decide type
}

#[derive(Debug)]
pub struct CancelAllOrders;

#[derive(Debug)]
pub struct GetBalance {
    pub asset: Option<Asset>,
}

#[derive(Debug)]
pub struct GetPosition {
    pub market: Market,
}
