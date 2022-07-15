//! Common types across various exchanges.

use std::{convert::Infallible, future::Future, pin::Pin, str::FromStr};

use rust_decimal::Decimal;
use thiserror::Error;

use crate::ReadyCall;

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
    pub ticks: Option<u64>,
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

pub type BoxedServiceFuture<'a, S, Request> =
    Pin<Box<dyn Future<Output = <<S as tower::Service<Request>>::Future as Future>::Output> + 'a>>;

/// A special type to indicate request is unsupported, used on [`CommonOpsService`]'s associated type
pub type Unsupported = Infallible;

/// Constraints to ensure that a service support [`tower::Service`] for common requests
pub trait CommonOpsService:
    Sized
    + tower::Service<Self::GetTradesRequest>
    + tower::Service<Self::GetOrderbookRequest>
    + tower::Service<Self::GetOrdersRequest>
    + tower::Service<Self::GetAllOrdersRequest>
    + tower::Service<Self::PlaceOrderRequest>
    + tower::Service<Self::CancelOrderRequest>
    + tower::Service<Self::CancelAllOrdersRequest>
    + tower::Service<Self::GetBalanceRequest>
    + tower::Service<Self::GetPositionRequest>
where
    <Self as tower::Service<Self::GetTradesRequest>>::Error:
        From<<Self as CommonOpsService>::Error>,
    <Self as tower::Service<Self::GetOrderbookRequest>>::Error:
        From<<Self as CommonOpsService>::Error>,
    <Self as tower::Service<Self::GetOrdersRequest>>::Error:
        From<<Self as CommonOpsService>::Error>,
    <Self as tower::Service<Self::GetAllOrdersRequest>>::Error:
        From<<Self as CommonOpsService>::Error>,
    <Self as tower::Service<Self::PlaceOrderRequest>>::Error:
        From<<Self as CommonOpsService>::Error>,
    <Self as tower::Service<Self::CancelOrderRequest>>::Error:
        From<<Self as CommonOpsService>::Error>,
    <Self as tower::Service<Self::CancelAllOrdersRequest>>::Error:
        From<<Self as CommonOpsService>::Error>,
    <Self as tower::Service<Self::GetBalanceRequest>>::Error:
        From<<Self as CommonOpsService>::Error>,
    <Self as tower::Service<Self::GetPositionRequest>>::Error:
        From<<Self as CommonOpsService>::Error>,
{
    type Error;
    type GetTradesRequest: TryFrom<GetTrades, Error = <Self as CommonOpsService>::Error>;
    type GetOrderbookRequest: TryFrom<GetOrderbook, Error = <Self as CommonOpsService>::Error>;
    type GetOrdersRequest: TryFrom<GetOrders, Error = <Self as CommonOpsService>::Error>;
    type GetAllOrdersRequest: TryFrom<GetAllOrders, Error = <Self as CommonOpsService>::Error>;
    type PlaceOrderRequest: TryFrom<PlaceOrder, Error = <Self as CommonOpsService>::Error>;
    type CancelOrderRequest: TryFrom<CancelOrder, Error = <Self as CommonOpsService>::Error>;
    type CancelAllOrdersRequest: TryFrom<CancelAllOrders, Error = <Self as CommonOpsService>::Error>;
    type GetBalanceRequest: TryFrom<GetBalance, Error = <Self as CommonOpsService>::Error>;
    type GetPositionRequest: TryFrom<GetPosition, Error = <Self as CommonOpsService>::Error>;

    fn get_trades(&mut self, market: Market) -> BoxedServiceFuture<Self, Self::GetTradesRequest> {
        Box::pin(async move {
            self.ready_call(<Self::GetTradesRequest>::try_from(GetTrades { market })?)
                .await
        })
    }
    fn get_orderbook(
        &mut self,
        market: Market,
        ticks: Option<u64>,
    ) -> BoxedServiceFuture<Self, Self::GetOrderbookRequest> {
        Box::pin(async move {
            self.ready_call(<Self::GetOrderbookRequest>::try_from(GetOrderbook {
                market,
                ticks,
            })?)
            .await
        })
    }
    fn get_orders(&mut self, market: Market) -> BoxedServiceFuture<Self, Self::GetOrdersRequest> {
        Box::pin(async move {
            self.ready_call(<Self::GetOrdersRequest>::try_from(GetOrders { market })?)
                .await
        })
    }
    fn get_all_orders(&mut self) -> BoxedServiceFuture<Self, Self::GetAllOrdersRequest> {
        Box::pin(async move {
            self.ready_call(<Self::GetAllOrdersRequest>::try_from(GetAllOrders)?)
                .await
        })
    }
    fn place_order(
        &mut self,
        market: Market,
        order: Order,
    ) -> BoxedServiceFuture<Self, Self::PlaceOrderRequest> {
        Box::pin(async move {
            self.ready_call(<Self::PlaceOrderRequest>::try_from(PlaceOrder {
                market,
                order,
            })?)
            .await
        })
    }
    fn cancel_order(
        &mut self,
        order_id: String,
    ) -> BoxedServiceFuture<Self, Self::CancelOrderRequest> {
        Box::pin(async move {
            self.ready_call(<Self::CancelOrderRequest>::try_from(CancelOrder {
                order_id,
            })?)
            .await
        })
    }
    fn cancel_all_orders(&mut self) -> BoxedServiceFuture<Self, Self::CancelAllOrdersRequest> {
        Box::pin(async move {
            self.ready_call(<Self::CancelAllOrdersRequest>::try_from(CancelAllOrders)?)
                .await
        })
    }
    fn get_balance(
        &mut self,
        asset: Option<Asset>,
    ) -> BoxedServiceFuture<Self, Self::GetBalanceRequest> {
        Box::pin(async move {
            self.ready_call(<Self::GetBalanceRequest>::try_from(GetBalance { asset })?)
                .await
        })
    }
    fn get_position(
        &mut self,
        market: Market,
    ) -> BoxedServiceFuture<Self, Self::GetPositionRequest> {
        Box::pin(async move {
            self.ready_call(<Self::GetPositionRequest>::try_from(GetPosition {
                market,
            })?)
            .await
        })
    }
}
