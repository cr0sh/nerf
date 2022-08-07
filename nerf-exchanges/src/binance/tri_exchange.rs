//! A hack-ish way to support [`CommonOps`] by converting given common request type into [`TriExchange`].
//!
//! [`CommonOps`]: crate::common::CommonOps

use nerf::{HttpRequest, Request};
use serde::{Deserialize, Serialize};

use crate::common::{
    CancelOrder, GetOrderbook, GetOrders, GetPosition, GetTrades, IntoCommon, MarketKind,
    PlaceOrder, Unsupported,
};

use super::{Signer, __private::Sealed};

/// Dynamic routing of request/responses between `{api,fapi,dapi}.binance.com`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TriExchange<Spot, Futures, Inverse> {
    Spot(Spot),
    Futures(Futures),
    Inverse(Inverse),
}

pub trait ExtractMarketKind {
    fn market_kind(&self) -> MarketKind;
}

impl ExtractMarketKind for GetTrades {
    fn market_kind(&self) -> MarketKind {
        *self.market.kind()
    }
}

impl ExtractMarketKind for GetOrderbook {
    fn market_kind(&self) -> MarketKind {
        *self.market.kind()
    }
}

impl ExtractMarketKind for GetOrders {
    fn market_kind(&self) -> MarketKind {
        *self.market.kind()
    }
}

impl ExtractMarketKind for PlaceOrder {
    fn market_kind(&self) -> MarketKind {
        *self.market.kind()
    }
}

impl ExtractMarketKind for CancelOrder {
    fn market_kind(&self) -> MarketKind {
        *self.market.kind()
    }
}

impl ExtractMarketKind for GetPosition {
    fn market_kind(&self) -> MarketKind {
        *self.market.kind()
    }
}

impl ExtractMarketKind for Unsupported {
    fn market_kind(&self) -> MarketKind {
        match *self {}
    }
}

impl<T: ExtractMarketKind, Spot, Futures, Inverse> From<T> for TriExchange<Spot, Futures, Inverse>
where
    Spot: From<T>,
    Futures: From<T>,
    Inverse: From<T>,
{
    fn from(x: T) -> Self {
        match x.market_kind() {
            MarketKind::Spot => Self::Spot(x.into()),
            MarketKind::UsdMarginedPerpetual => Self::Futures(x.into()),
            MarketKind::CoinMarginedPerpetual => Self::Inverse(x.into()),
        }
    }
}

impl<Spot, Futures, Inverse> Sealed for TriExchange<Spot, Futures, Inverse>
where
    Spot: Sealed,
    Futures: Sealed,
    Inverse: Sealed,
{
}

impl<Spot, Futures, Inverse> Request for TriExchange<Spot, Futures, Inverse>
where
    Spot: Request,
    Futures: Request,
    Inverse: Request,
{
    type Response = TriExchange<Spot::Response, Futures::Response, Inverse::Response>;
}

impl<Spot, Futures, Inverse> HttpRequest for TriExchange<Spot, Futures, Inverse>
where
    Spot: HttpRequest,
    Futures: HttpRequest,
    Inverse: HttpRequest,
{
    fn uri(&self) -> hyper::http::Uri {
        match self {
            TriExchange::Spot(x) => x.uri(),
            TriExchange::Futures(x) => x.uri(),
            TriExchange::Inverse(x) => x.uri(),
        }
    }

    fn method(&self) -> hyper::http::Method {
        match self {
            TriExchange::Spot(x) => x.method(),
            TriExchange::Futures(x) => x.method(),
            TriExchange::Inverse(x) => x.method(),
        }
    }
}

// TODO: implement dapi and introduce `Inverse` tyvar here
impl<Spot, Futures> Signer for TriExchange<Spot, Futures, Unsupported>
where
    Spot: Signer,
    Futures: Signer<Signer = Spot::Signer>,
{
    type Signer = Spot::Signer;
}

// TODO: implement dapi and introduce `Inverse` tyvar here
impl<Spot, Futures> IntoCommon for TriExchange<Spot, Futures, Unsupported>
where
    Spot: IntoCommon,
    Futures: IntoCommon<Output = Spot::Output>,
{
    type Output = Spot::Output;

    fn into_common(self) -> Self::Output {
        match self {
            TriExchange::Spot(x) => x.into_common(),
            TriExchange::Futures(x) => x.into_common(),
            TriExchange::Inverse(..) => unreachable!(),
        }
    }
}
