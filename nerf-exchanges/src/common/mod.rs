//! Common types across various exchanges.

mod types;

use std::{convert::Infallible, fmt::Display, future::Future, pin::Pin, str::FromStr};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use nerf::{ClientService, ReadyCall};
use tower::buffer::Buffer;

pub use self::types::*;

pub type Asset = String;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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

impl<T: AsRef<str>> From<T> for Market {
    /// [`FromStr`] shorthand which panicks on failure.
    fn from(x: T) -> Self {
        let x = x.as_ref();
        x.parse().expect(x)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
            "swap" => Ok(MarketKind::UsdMarginedPerpetual),
            "inverse" => Ok(MarketKind::CoinMarginedPerpetual),
            other => Err(MarketParseError::InvalidKind(other.to_string())),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Order {
    Market {
        side: Side,
        quantity: Decimal,
    },
    Limit {
        side: Side,
        quantity: Decimal,
        price: Decimal,
        time_in_force: TimeInForce,
    },
    StopMarket {
        side: Side,
        stop_price: Decimal,
        quantity: Decimal,
    },
    StopLimit {
        side: Side,
        stop_price: Decimal,
        quantity: Decimal,
        price: Decimal,
        time_in_force: TimeInForce,
    },
}

impl Order {
    /// Returns the side of this [`Order`].
    pub fn side(&self) -> Side {
        match self {
            Order::Market { side, .. } => *side,
            Order::Limit { side, .. } => *side,
            Order::StopMarket { side, .. } => *side,
            Order::StopLimit { side, .. } => *side,
        }
    }

    /// Returns the *time in force* of this [`Order`] if applicable.
    pub fn time_in_force(&self) -> Option<TimeInForce> {
        match self {
            Order::Market { .. } => None,
            Order::Limit { time_in_force, .. } => Some(*time_in_force),
            Order::StopMarket { .. } => None,
            Order::StopLimit { time_in_force, .. } => Some(*time_in_force),
        }
    }

    /// Returns the quantity of this [`Order`].
    pub fn quantity(&self) -> Decimal {
        match self {
            Order::Market { quantity, .. } => *quantity,
            Order::Limit { quantity, .. } => *quantity,
            Order::StopMarket { quantity, .. } => *quantity,
            Order::StopLimit { quantity, .. } => *quantity,
        }
    }

    /// Returns the price of this [`Order`] if applicable.
    pub fn price(&self) -> Option<Decimal> {
        match self {
            Order::Market { .. } => None,
            Order::Limit { price, .. } => Some(*price),
            Order::StopMarket { .. } => None,
            Order::StopLimit { price, .. } => Some(*price),
        }
    }

    /// Returns the stop price of this [`Order`] if applicable.
    pub fn stop_price(&self) -> Option<Decimal> {
        match self {
            Order::Market { .. } => None,
            Order::Limit { .. } => None,
            Order::StopMarket { stop_price, .. } => Some(*stop_price),
            Order::StopLimit { stop_price, .. } => Some(*stop_price),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[non_exhaustive]
pub enum TimeInForce {
    GoodTilCancled,
    ImmediateOrCancel,
    FillOrKill,
    GoodTilCrossing,
}

#[derive(Debug)]
pub struct GetTickers {
    pub symbols: Option<Vec<Market>>,
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
    pub reduce_only: bool, // only applicable in futures market
}

#[derive(Debug)]
pub struct CancelOrder {
    pub market: Market,
    pub order_id: String, // TODO: decide type
}

#[derive(Debug)]
pub struct CancelAllOrders;

#[derive(Debug)]
pub struct GetBalance;

#[derive(Debug)]
pub struct GetPosition {
    pub market: Market,
}

pub type BoxedServiceFuture<'a, S, Request> =
    Pin<Box<dyn Future<Output = <<S as tower::Service<Request>>::Future as Future>::Output> + 'a>>;

/// A special type to indicate request is unsupported, used on [`CommonOpsService`]'s associated type
///
/// May be migrated into alias of `!` once the `never` type is stabilized.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Unsupported {}

// impl<T> From<T> for Unsupported {
//     fn from(x: T) -> Self {
//         panic!("Unsupported request");
//     }
// }

impl Future for Unsupported {
    type Output = Result<Infallible, Infallible>;

    fn poll(
        self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match *self.get_mut() {}
    }
}

impl nerf::Request for Unsupported {
    type Response = Unsupported;
}

impl nerf::HttpRequest for Unsupported {
    fn uri(&self) -> hyper::http::Uri {
        match *self {}
    }

    fn method(&self) -> hyper::http::Method {
        match *self {}
    }
}

// impl<T> tower::Service<Unsupported> for ClientService<T> {
//     type Response = Infallible;

//     type Error = Infallible;

//     type Future = Unsupported;

//     fn poll_ready(
//         &mut self,
//         _cx: &mut std::task::Context<'_>,
//     ) -> std::task::Poll<Result<(), Self::Error>> {
//         panic!("Unsupported request type")
//     }

//     fn call(&mut self, req: Unsupported) -> Self::Future {
//         match req {}
//     }
// }

/// A trait to mark if request endpoints are public or not.
pub trait Signer {
    type Signer: SignerKind;
}

impl Signer for Unsupported {
    type Signer = Disabled;
}

pub struct Disabled;
pub struct Private;

/// A trait to extract signer kind on run-time.
pub trait SignerKind {
    fn is_private() -> bool;
}

impl SignerKind for Disabled {
    fn is_private() -> bool {
        false
    }
}

impl SignerKind for Private {
    fn is_private() -> bool {
        true
    }
}

pub trait CommonOps {
    type GetTickersRequest: TryFrom<GetTickers>;
    type GetTradesRequest: TryFrom<GetTrades>;
    type GetOrderbookRequest: TryFrom<GetOrderbook>;
    type GetOrdersRequest: TryFrom<GetOrders>;
    type GetAllOrdersRequest: TryFrom<GetAllOrders>;
    type PlaceOrderRequest: TryFrom<PlaceOrder>;
    type CancelOrderRequest: TryFrom<CancelOrder>;
    type CancelAllOrdersRequest: TryFrom<CancelAllOrders>;
    type GetBalanceRequest: TryFrom<GetBalance>;
    type GetPositionRequest: TryFrom<GetPosition>;
}

impl<T> CommonOps for ClientService<T>
where
    T: CommonOps,
{
    type GetTickersRequest = <T as CommonOps>::GetTickersRequest;

    type GetTradesRequest = <T as CommonOps>::GetTradesRequest;

    type GetOrderbookRequest = <T as CommonOps>::GetOrderbookRequest;

    type GetOrdersRequest = <T as CommonOps>::GetOrdersRequest;

    type GetAllOrdersRequest = <T as CommonOps>::GetAllOrdersRequest;

    type PlaceOrderRequest = <T as CommonOps>::PlaceOrderRequest;

    type CancelOrderRequest = <T as CommonOps>::CancelOrderRequest;

    type CancelAllOrdersRequest = <T as CommonOps>::CancelAllOrdersRequest;

    type GetBalanceRequest = <T as CommonOps>::GetBalanceRequest;

    type GetPositionRequest = <T as CommonOps>::GetPositionRequest;
}

/// Constraints to ensure that a service support [`tower::Service`] for common requests
pub trait CommonOpsService:
    CommonOps
    + tower::Service<Self::GetTickersRequest>
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
        From<<Self::GetTradesRequest as TryFrom<GetTrades>>::Error>,
    <Self as tower::Service<Self::GetOrderbookRequest>>::Error:
        From<<Self::GetOrderbookRequest as TryFrom<GetOrderbook>>::Error>,
    <Self as tower::Service<Self::GetOrdersRequest>>::Error:
        From<<Self::GetOrdersRequest as TryFrom<GetOrders>>::Error>,
    <Self as tower::Service<Self::GetAllOrdersRequest>>::Error:
        From<<Self::GetAllOrdersRequest as TryFrom<GetAllOrders>>::Error>,
    <Self as tower::Service<Self::PlaceOrderRequest>>::Error:
        From<<Self::PlaceOrderRequest as TryFrom<PlaceOrder>>::Error>,
    <Self as tower::Service<Self::CancelOrderRequest>>::Error:
        From<<Self::CancelOrderRequest as TryFrom<CancelOrder>>::Error>,
    <Self as tower::Service<Self::CancelAllOrdersRequest>>::Error:
        From<<Self::CancelAllOrdersRequest as TryFrom<CancelAllOrders>>::Error>,
    <Self as tower::Service<Self::GetBalanceRequest>>::Error:
        From<<Self::GetBalanceRequest as TryFrom<GetBalance>>::Error>,
    <Self as tower::Service<Self::GetPositionRequest>>::Error:
        From<<Self::GetPositionRequest as TryFrom<GetPosition>>::Error>,
{
    fn get_tickers(
        &mut self,
        symbols: Option<impl Into<Vec<Market>>>,
    ) -> BoxedServiceFuture<Self, Self::GetTickersRequest>;
    fn get_trades(
        &mut self,
        market: impl Into<Market>,
    ) -> BoxedServiceFuture<Self, Self::GetTradesRequest>;
    fn get_orderbook(
        &mut self,
        market: impl Into<Market>,
        ticks: Option<u64>,
    ) -> BoxedServiceFuture<Self, Self::GetOrderbookRequest>;
    fn get_orders(&mut self, market: Market) -> BoxedServiceFuture<Self, Self::GetOrdersRequest>;
    fn get_all_orders(&mut self) -> BoxedServiceFuture<Self, Self::GetAllOrdersRequest>;
    fn place_order(
        &mut self,
        market: impl Into<Market>,
        order: Order,
        reduce_only: bool, // only applicable in futures market
    ) -> BoxedServiceFuture<Self, Self::PlaceOrderRequest>;
    fn cancel_order(
        &mut self,
        market: impl Into<Market>,
        order_id: String,
    ) -> BoxedServiceFuture<Self, Self::CancelOrderRequest>;
    fn cancel_all_orders(&mut self) -> BoxedServiceFuture<Self, Self::CancelAllOrdersRequest>;
    fn get_balance(&mut self) -> BoxedServiceFuture<Self, Self::GetBalanceRequest>;
    fn get_position(
        &mut self,
        market: impl Into<Market>,
    ) -> BoxedServiceFuture<Self, Self::GetPositionRequest>;
}

impl<T> CommonOpsService for T
where
    T: CommonOps
        + tower::Service<Self::GetTickersRequest>
        + tower::Service<Self::GetTradesRequest>
        + tower::Service<Self::GetOrderbookRequest>
        + tower::Service<Self::GetOrdersRequest>
        + tower::Service<Self::GetAllOrdersRequest>
        + tower::Service<Self::PlaceOrderRequest>
        + tower::Service<Self::CancelOrderRequest>
        + tower::Service<Self::CancelAllOrdersRequest>
        + tower::Service<Self::GetBalanceRequest>
        + tower::Service<Self::GetPositionRequest>,
    <T as tower::Service<T::GetTickersRequest>>::Error:
        From<<T::GetTickersRequest as TryFrom<GetTickers>>::Error>,
    <T as tower::Service<T::GetTradesRequest>>::Error:
        From<<T::GetTradesRequest as TryFrom<GetTrades>>::Error>,
    <T as tower::Service<T::GetOrderbookRequest>>::Error:
        From<<T::GetOrderbookRequest as TryFrom<GetOrderbook>>::Error>,
    <T as tower::Service<T::GetOrdersRequest>>::Error:
        From<<T::GetOrdersRequest as TryFrom<GetOrders>>::Error>,
    <T as tower::Service<T::GetAllOrdersRequest>>::Error:
        From<<T::GetAllOrdersRequest as TryFrom<GetAllOrders>>::Error>,
    <T as tower::Service<T::PlaceOrderRequest>>::Error:
        From<<T::PlaceOrderRequest as TryFrom<PlaceOrder>>::Error>,
    <T as tower::Service<T::CancelOrderRequest>>::Error:
        From<<T::CancelOrderRequest as TryFrom<CancelOrder>>::Error>,
    <T as tower::Service<T::CancelAllOrdersRequest>>::Error:
        From<<T::CancelAllOrdersRequest as TryFrom<CancelAllOrders>>::Error>,
    <T as tower::Service<T::GetBalanceRequest>>::Error:
        From<<T::GetBalanceRequest as TryFrom<GetBalance>>::Error>,
    <T as tower::Service<T::GetPositionRequest>>::Error:
        From<<T::GetPositionRequest as TryFrom<GetPosition>>::Error>,
{
    fn get_tickers(
        &mut self,
        symbols: Option<impl Into<Vec<Market>>>,
    ) -> BoxedServiceFuture<Self, Self::GetTickersRequest> {
        let symbols = symbols.map(Into::into);
        Box::pin(async move {
            self.ready_call(<Self::GetTickersRequest>::try_from(GetTickers { symbols })?)
                .await
        })
    }
    fn get_trades(
        &mut self,
        market: impl Into<Market>,
    ) -> BoxedServiceFuture<Self, Self::GetTradesRequest> {
        let market = market.into();
        Box::pin(async move {
            self.ready_call(<Self::GetTradesRequest>::try_from(GetTrades { market })?)
                .await
        })
    }
    fn get_orderbook(
        &mut self,
        market: impl Into<Market>,
        ticks: Option<u64>,
    ) -> BoxedServiceFuture<Self, Self::GetOrderbookRequest> {
        let market = market.into();
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
        market: impl Into<Market>,
        order: Order,
        reduce_only: bool,
    ) -> BoxedServiceFuture<Self, Self::PlaceOrderRequest> {
        let market = market.into();
        Box::pin(async move {
            self.ready_call(<Self::PlaceOrderRequest>::try_from(PlaceOrder {
                market,
                order,
                reduce_only,
            })?)
            .await
        })
    }
    fn cancel_order(
        &mut self,
        market: impl Into<Market>,
        order_id: String,
    ) -> BoxedServiceFuture<Self, Self::CancelOrderRequest> {
        let market = market.into();
        Box::pin(async move {
            self.ready_call(<Self::CancelOrderRequest>::try_from(CancelOrder {
                market,
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
    fn get_balance(&mut self) -> BoxedServiceFuture<Self, Self::GetBalanceRequest> {
        Box::pin(async move {
            self.ready_call(<Self::GetBalanceRequest>::try_from(GetBalance)?)
                .await
        })
    }
    fn get_position(
        &mut self,
        market: impl Into<Market>,
    ) -> BoxedServiceFuture<Self, Self::GetPositionRequest> {
        let market = market.into();
        Box::pin(async move {
            self.ready_call(<Self::GetPositionRequest>::try_from(GetPosition {
                market,
            })?)
            .await
        })
    }
}

impl<T, Request> CommonOps for Buffer<T, Request>
where
    T: CommonOps + tower::Service<Request>,
{
    type GetTickersRequest = <T as CommonOps>::GetTickersRequest;

    type GetTradesRequest = <T as CommonOps>::GetTradesRequest;

    type GetOrderbookRequest = <T as CommonOps>::GetOrderbookRequest;

    type GetOrdersRequest = <T as CommonOps>::GetOrdersRequest;

    type GetAllOrdersRequest = <T as CommonOps>::GetAllOrdersRequest;

    type PlaceOrderRequest = <T as CommonOps>::PlaceOrderRequest;

    type CancelOrderRequest = <T as CommonOps>::CancelOrderRequest;

    type CancelAllOrdersRequest = <T as CommonOps>::CancelAllOrdersRequest;

    type GetBalanceRequest = <T as CommonOps>::GetBalanceRequest;

    type GetPositionRequest = <T as CommonOps>::GetPositionRequest;
}

macro_rules! impl_unsupported {
    ($name:ident, $($others:ident$(,)?)* ) => {
        impl From<$name> for $crate::common::Unsupported {
            fn from(_: $name) -> Self {
                panic!("Unsupported request type");
            }
        }

        impl_unsupported!($($others ,)*);
    };

    () => {}
}

impl_unsupported!(
    GetTickers,
    GetTrades,
    GetOrderbook,
    GetOrders,
    GetAllOrders,
    PlaceOrder,
    CancelOrder,
    CancelAllOrders,
    GetBalance,
    GetPosition,
);
