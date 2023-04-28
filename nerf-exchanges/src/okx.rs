use std::{collections::HashMap, fmt::Debug, future::Future, pin::Pin};

use crate::{
    common::{self, Disabled, Signer, Unsupported},
    ts_milliseconds_str, Error,
};
use __private::Sealed;

use chrono::{DateTime, Utc};
use http::Method;
use nerf::{get, tag, Client, HttpRequest, Request};
use rust_decimal::Decimal;
use serde::{
    de::{DeserializeOwned, IntoDeserializer},
    Deserialize, Deserializer, Serialize,
};
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Clone, Debug, Serialize)]
#[get("https://aws.okx.com/api/v5/market/ticker", response = (GetV5MarketTickerResponseItem,))]
#[tag(Signer = Disabled)]
#[serde(rename_all = "camelCase")]
pub struct GetV5MarketTicker {
    pub inst_id: String,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InstType {
    Spot,
    Swap,
    Futures,
    Option,
}

#[skip_serializing_none]
#[derive(Clone, Debug, Serialize)]
#[get("https://aws.okx.com/api/v5/market/tickers", response = Vec<GetV5MarketTickerResponseItem>)]
#[tag(Signer = Disabled)]
#[serde(rename_all = "camelCase")]
pub struct GetV5MarketTickers {
    pub inst_type: InstType,
    #[serde(rename = "uly")]
    pub underlying: Option<String>,
    pub inst_family: Option<String>,
}

fn empty_as_zero<'de, D>(deserializer: D) -> Result<Decimal, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s.is_empty() {
        return Ok(Decimal::ZERO);
    }
    let deserializer = s.into_deserializer();
    <Decimal as Deserialize>::deserialize(deserializer)
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetV5MarketTickerResponseItem {
    pub inst_type: String,
    pub inst_id: String,
    pub last: Decimal,
    pub last_sz: Decimal,
    #[serde(deserialize_with = "empty_as_zero")]
    pub ask_px: Decimal,
    pub ask_sz: Decimal,
    #[serde(deserialize_with = "empty_as_zero")]
    pub bid_px: Decimal,
    pub bid_sz: Decimal,
    #[serde(with = "ts_milliseconds_str")]
    pub ts: DateTime<Utc>,
}

#[skip_serializing_none]
#[derive(Clone, Debug, Serialize)]
#[get("https://aws.okx.com/api/v5/market/books", response = (GetV5MarketBooksResponse,))]
#[tag(Signer = Disabled)]
#[serde(rename_all = "camelCase")]
pub struct GetV5MarketBooks {
    inst_id: String,
    sz: Option<u64>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetV5MarketBooksResponse {
    pub asks: Vec<GetV5MarketBooksResponseItem>,
    pub bids: Vec<GetV5MarketBooksResponseItem>,
    #[serde(with = "ts_milliseconds_str")]
    pub ts: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct GetV5MarketBooksResponseItem {
    pub price: Decimal,
    pub quantity: Decimal,
    pub num_orders: u64,
}

impl<'de> Deserialize<'de> for GetV5MarketBooksResponseItem {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (price, quantity, _, num_orders) =
            <(Decimal, Decimal, Decimal, String)>::deserialize(deserializer)?;
        let num_orders = num_orders
            .parse::<u64>()
            .map_err(|e| serde::de::Error::custom(e.to_string()))?;
        Ok(Self {
            price,
            quantity,
            num_orders,
        })
    }
}

#[derive(Clone, Debug)]
pub struct OkxClient<S>(S);

impl<S> OkxClient<S> {
    pub fn new(x: S) -> Self {
        Self(x)
    }
}

impl<T, S> Client<T> for OkxClient<S>
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
        #[derive(Debug, Deserialize)]
        struct OkxResponse<T> {
            data: T,
        }

        #[derive(Debug, Deserialize)]
        struct OkxError {
            code: String,
            msg: String,
        }

        if x.status().is_success() {
            Box::pin(async {
                let resp: OkxResponse<T::Response> = serde_json::from_reader(
                    hyper::body::Buf::reader(hyper::body::aggregate(x).await?),
                )
                .map_err(Error::DeserializeJsonBody)?;
                Ok(resp.data)
            })
        } else {
            Box::pin(async {
                let resp: OkxError = serde_json::from_reader(hyper::body::Buf::reader(
                    hyper::body::aggregate(x).await?,
                ))
                .map_err(Error::DeserializeJsonBody)?;
                Err(Error::RequestFailed {
                    code: Some(resp.code),
                    msg: Some(resp.msg),
                })
            })
        }
    }
}

impl<S> tower::Service<Unsupported> for OkxClient<S> {
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

impl From<common::GetTickers> for GetV5MarketTickers {
    fn from(_: common::GetTickers) -> Self {
        Self {
            inst_type: InstType::Spot, // NOTE: only spot tickers are supported
            underlying: None,
            inst_family: None,
        }
    }
}

impl From<common::GetOrderbook> for GetV5MarketBooks {
    fn from(x: common::GetOrderbook) -> Self {
        let inst_id = match x.market.kind() {
            common::MarketKind::Spot => format!("{}-{}", x.market.base(), x.market.quote()),
            common::MarketKind::UsdMarginedPerpetual => {
                format!("{}-{}-SWAP", x.market.base(), x.market.quote())
            }
            common::MarketKind::CoinMarginedPerpetual => todo!(),
        };
        Self {
            inst_id,
            sz: x.ticks,
        }
    }
}

impl common::IntoCommon for Vec<GetV5MarketTickerResponseItem> {
    type Output = HashMap<common::Market, common::Ticker>;

    fn into_common(self) -> Self::Output {
        self.into_iter()
            .filter_map(|x| {
                let (base, quote) = x.inst_id.split_once('-')?;
                Some((
                    format!("spot:{base}/{quote}").into(),
                    common::Ticker::new(x.bid_px, x.ask_px, None),
                ))
            })
            .collect()
    }
}

impl common::IntoCommon for GetV5MarketBooksResponse {
    type Output = common::Orderbook;

    fn into_common(self) -> Self::Output {
        common::Orderbook::new(
            self.bids
                .iter()
                .map(|x| common::OrderbookItem {
                    price: x.price,
                    quantity: x.quantity,
                })
                .collect(),
            self.asks
                .iter()
                .map(|x| common::OrderbookItem {
                    price: x.price,
                    quantity: x.quantity,
                })
                .collect(),
            Some(self.ts),
        )
    }
}

impl<S> common::CommonOps for OkxClient<S> {
    type GetTickersRequest = GetV5MarketTickers;

    type GetTradesRequest = Unsupported;

    type GetOrderbookRequest = GetV5MarketBooks;

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
