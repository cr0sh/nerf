use std::{fmt::Debug, future::Future, pin::Pin};

use crate::{
    common::{self, Disabled, Signer, Unsupported},
    ts_milliseconds_str, Error,
};
use __private::Sealed;

use chrono::{DateTime, Utc};
use http::Method;
use nerf::{get, tag, Client, HttpRequest, Request};
use rust_decimal::Decimal;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Clone, Debug, Serialize)]
#[get("https://aws.okx.com/api/v5/market/ticker", response = (GetV5MarketTickerResponseItem,))]
#[tag(Signer = Disabled)]
#[serde(rename_all = "camelCase")]
pub struct GetV5MarketTicker {
    pub inst_id: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetV5MarketTickerResponseItem {
    pub inst_type: String,
    pub inst_id: String,
    pub last: Decimal,
    pub last_sz: Decimal,
    pub ask_px: Decimal,
    pub ask_sz: Decimal,
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

    type TryFromResponseFuture = Pin<Box<dyn Future<Output = Result<T::Response, Self::Error>>>>;

    fn service(&mut self) -> &mut Self::Service {
        &mut self.0
    }

    fn try_into_request(&mut self, x: T) -> Result<hyper::Request<hyper::Body>, Self::Error> {
        let query = serde_urlencoded::to_string(&x)
            .map_err(Error::SerializeUrlencodedBody)?
            .replace("%5B", "[")
            .replace("%5D", "]");
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

impl From<common::GetTickers> for GetV5MarketTicker {
    fn from(x: common::GetTickers) -> Self {
        let market = x
            .symbols
            .and_then(|x| x.first().cloned())
            .expect("OKX only supports single ticker");
        Self {
            inst_id: format!("{}-{}", market.base(), market.quote()),
        }
    }
}

impl From<common::GetOrderbook> for GetV5MarketBooks {
    fn from(x: common::GetOrderbook) -> Self {
        Self {
            inst_id: format!("{}-{}", x.market.base(), x.market.quote()),
            sz: x.ticks,
        }
    }
}

impl common::IntoCommon for (GetV5MarketTickerResponseItem,) {
    type Output = common::Ticker;

    fn into_common(self) -> Self::Output {
        common::Ticker::new(self.0.bid_px, self.0.ask_px)
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
        )
    }
}

impl<S> common::CommonOps for OkxClient<S> {
    type GetTickersRequest = GetV5MarketTicker;

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
