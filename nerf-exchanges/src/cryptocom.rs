use std::{collections::HashMap, fmt::Debug, future::Future, pin::Pin};

use crate::{
    common::{self, Disabled, Signer, Unsupported},
    Error,
};
use __private::Sealed;

use chrono::{serde::ts_milliseconds, DateTime, Utc};
use http::Method;
use nerf::{get, tag, Client, HttpRequest, Request};
use rust_decimal::Decimal;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Side {
    Buy,
    Sell,
}

#[skip_serializing_none]
#[derive(Clone, Debug, Serialize)]
#[get("https://api.crypto.com/v2/public/get-ticker", response = Vec<GetPublicGetTickerResponseItem>)]
#[tag(Signer = Disabled)]
pub struct GetPublicGetTicker {
    pub instrument_name: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GetPublicGetTickerResponseItem {
    #[serde(rename = "h")]
    pub highest_price_24h: Option<Decimal>,
    #[serde(rename = "l")]
    pub lowest_price_24h: Option<Decimal>, // null if there weren't any trades
    #[serde(rename = "a")]
    pub latest_trade_price: Option<Decimal>,
    #[serde(rename = "i")]
    pub instrument_name: String,
    #[serde(rename = "v")]
    pub volume_24h: Decimal,
    #[serde(rename = "vv")]
    pub volume_24h_usd: Decimal,
    #[serde(rename = "oi")]
    pub open_interest: Option<Decimal>,
    #[serde(rename = "c")]
    pub price_change_24h: Option<Decimal>,
    #[serde(rename = "b")]
    pub best_bid: Option<Decimal>, // null if there aren't any bids
    #[serde(rename = "k")]
    pub best_ask: Option<Decimal>, // null if there aren't any asks
    #[serde(rename = "t", with = "ts_milliseconds")]
    pub timestamp: DateTime<Utc>,
}

#[skip_serializing_none]
#[derive(Clone, Debug, Serialize)]
#[get("https://api.crypto.com/v2/public/get-trades", response = Vec<GetPublicGetTradesResponse>)]
#[tag(Signer = Disabled)]
pub struct GetPublicGetTrades {
    pub instrument_name: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GetPublicGetTradesResponse {
    #[serde(rename = "p")]
    pub price: Decimal,
    #[serde(rename = "q")]
    pub quantity: Decimal,
    #[serde(rename = "s")]
    pub side: Side,
    #[serde(rename = "i")]
    pub instrument_name: String,
    #[serde(rename = "t", with = "ts_milliseconds")]
    pub timestamp: DateTime<Utc>,
    #[serde(rename = "d")]
    pub id: String,
}

#[skip_serializing_none]
#[derive(Clone, Debug, Serialize)]
#[get("https://api.crypto.com/v2/public/get-book", response = (GetPublicGetBookResponse,))]
#[tag(Signer = Disabled)]
pub struct GetPublicGetBook {
    pub instrument_name: String,
    pub depth: Option<u64>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GetPublicGetBookResponse {
    pub asks: Vec<GetPublicGetBookResponseItem>,
    pub bids: Vec<GetPublicGetBookResponseItem>,
    #[serde(rename = "t", with = "ts_milliseconds")]
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct GetPublicGetBookResponseItem {
    pub price: Decimal,
    pub quantity: Decimal,
}

impl<'de> Deserialize<'de> for GetPublicGetBookResponseItem {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (price, quantity, _) = <(Decimal, Decimal, Decimal)>::deserialize(deserializer)?;
        Ok(Self { price, quantity })
    }
}

#[derive(Clone, Debug)]
pub struct CryptocomClient<S>(S);

impl<S> CryptocomClient<S> {
    pub fn new(x: S) -> Self {
        Self(x)
    }
}

impl<T, S> Client<T> for CryptocomClient<S>
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
        let query = serde_urlencoded::to_string(&x).map_err(Error::SerializeUrlencodedBody)?;
        if x.method() == Method::GET {
            let mut req = hyper::Request::new(hyper::Body::empty());
            let uri = x.uri();
            assert_eq!(uri.query(), None);
            req.headers_mut()
                .append("Accept", "application/json".parse().unwrap());
            *req.uri_mut() = format!("{}?{}", uri, query).parse().unwrap();
            Ok(req)
        } else {
            let mut req = hyper::Request::new(hyper::Body::from(query));
            let uri = x.uri();
            assert_eq!(uri.query(), None);
            req.headers_mut()
                .append("Accept", "application/json".parse().unwrap());
            *req.uri_mut() = uri;
            Ok(req)
        }
    }

    fn try_from_response(x: hyper::Response<hyper::Body>) -> Self::TryFromResponseFuture {
        #[derive(Clone, Debug, Deserialize)]
        struct CryptocomResponse<T> {
            pub data: T,
        }

        #[derive(Debug, Deserialize)]
        struct CryptocomError {
            code: String,
            message: String,
        }

        if x.status().is_success() {
            Box::pin(async {
                let resp: CryptocomResponse<T::Response> = serde_json::from_reader(
                    hyper::body::Buf::reader(hyper::body::aggregate(x).await?),
                )
                .map_err(Error::DeserializeJsonBody)?;
                Ok(resp.data)
            })
        } else {
            Box::pin(async {
                let resp: CryptocomError = serde_json::from_reader(hyper::body::Buf::reader(
                    hyper::body::aggregate(x).await?,
                ))
                .map_err(Error::DeserializeJsonBody)?;
                Err(Error::RequestFailed {
                    code: Some(resp.code),
                    msg: Some(resp.message),
                })
            })
        }
    }
}

impl<S> tower::Service<Unsupported> for CryptocomClient<S> {
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

impl From<common::GetTickers> for GetPublicGetTicker {
    fn from(_: common::GetTickers) -> Self {
        Self {
            instrument_name: None,
        }
    }
}

impl From<common::GetTrades> for GetPublicGetTrades {
    fn from(x: common::GetTrades) -> Self {
        Self {
            instrument_name: format!("{}_{}", x.market.base(), x.market.quote()),
        }
    }
}

impl From<common::GetOrderbook> for GetPublicGetBook {
    fn from(x: common::GetOrderbook) -> Self {
        Self {
            instrument_name: format!("{}_{}", x.market.base(), x.market.quote()),
            depth: x.ticks,
        }
    }
}

impl common::IntoCommon for Vec<GetPublicGetTickerResponseItem> {
    type Output = HashMap<common::Market, common::Ticker>;

    fn into_common(self) -> Self::Output {
        self.iter()
            .filter_map(|x| {
                let (base, quote) = x.instrument_name.split_once('_')?;
                Some((
                    format!("spot:{base}/{quote}").into(),
                    common::Ticker::new(
                        x.best_bid.expect("empty orderbook"),
                        x.best_ask.expect("empty orderbook"),
                    ),
                ))
            })
            .collect()
    }
}

impl common::IntoCommon for (GetPublicGetBookResponse,) {
    type Output = common::Orderbook;

    fn into_common(self) -> Self::Output {
        common::Orderbook::new(
            self.0
                .bids
                .iter()
                .map(|x| common::OrderbookItem {
                    price: x.price,
                    quantity: x.quantity,
                })
                .collect(),
            self.0
                .asks
                .iter()
                .map(|x| common::OrderbookItem {
                    price: x.price,
                    quantity: x.quantity,
                })
                .collect(),
        )
    }
}

impl<S> common::CommonOps for CryptocomClient<S> {
    type GetTickersRequest = GetPublicGetTicker;

    type GetTradesRequest = GetPublicGetTrades;

    type GetOrderbookRequest = GetPublicGetBook;

    type GetOrdersRequest = Unsupported;

    type GetAllOrdersRequest = Unsupported;

    type PlaceOrderRequest = Unsupported;

    type CancelOrderRequest = Unsupported;

    type CancelAllOrdersRequest = Unsupported;

    type GetBalanceRequest = Unsupported;

    type GetPositionRequest = Unsupported;
}

mod __private {
    use crate::common::Unsupported;

    pub trait Sealed {}
    impl Sealed for Unsupported {}
}
