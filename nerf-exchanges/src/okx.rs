use std::{collections::HashMap, fmt::Debug, future::Future, pin::Pin};

use crate::{
    common::{self, Disabled, Private, Signer, SignerKind, Unsupported},
    ts_milliseconds_str, Error,
};
use __private::Sealed;

use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use http::Method;
use nerf::{get, tag, Client, HttpRequest, Request};
use rust_decimal::Decimal;
use serde::{
    de::{DeserializeOwned, IntoDeserializer},
    Deserialize, Deserializer, Serialize,
};
use serde_with::skip_serializing_none;
use sha2::Sha256;

use base64::prelude::*;

#[skip_serializing_none]
#[derive(Clone, Debug, Serialize)]
#[get("https://aws.okx.com/api/v5/market/ticker", response = (GetV5MarketTickerResponseItem,))]
#[tag(Signer = Disabled)]
#[serde(rename_all = "camelCase")]
pub struct GetV5MarketTicker {
    pub inst_id: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
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

#[skip_serializing_none]
#[derive(Clone, Debug, Serialize)]
#[get("https://aws.okx.com/api/v5/account/balance", response = (GetV5AccountBalanceResponse,))]
#[tag(Signer = Private)]
pub struct GetV5AccountBalance {
    pub ccy: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetV5AccountBalanceResponse {
    #[serde(with = "ts_milliseconds_str")]
    pub u_time: DateTime<Utc>,
    pub total_eq: Decimal,
    #[serde(deserialize_with = "empty_as_zero")] // TODO: implement empty_as_none and use here
    pub iso_eq: Decimal,
    #[serde(deserialize_with = "empty_as_zero")]
    pub adj_eq: Decimal,
    #[serde(deserialize_with = "empty_as_zero")]
    pub ord_froz: Decimal,
    #[serde(deserialize_with = "empty_as_zero")]
    pub imr: Decimal,
    #[serde(deserialize_with = "empty_as_zero")]
    pub mmr: Decimal,
    #[serde(deserialize_with = "empty_as_zero")]
    pub mgn_ratio: Decimal,
    #[serde(deserialize_with = "empty_as_zero")]
    pub notional_usd: Decimal,
    pub details: Vec<GetV5AccountBalanceResponseDetails>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetV5AccountBalanceResponseDetails {
    pub ccy: String,
    pub eq: Decimal,
    #[serde(with = "ts_milliseconds_str")]
    pub u_time: DateTime<Utc>,
    #[serde(deserialize_with = "empty_as_zero")]
    pub iso_eq: Decimal,
    #[serde(deserialize_with = "empty_as_zero")]
    pub avail_eq: Decimal,
    #[serde(deserialize_with = "empty_as_zero")]
    pub dis_eq: Decimal,
    #[serde(deserialize_with = "empty_as_zero")]
    pub avail_bal: Decimal,
    #[serde(deserialize_with = "empty_as_zero")]
    pub frozen_bal: Decimal,
    #[serde(deserialize_with = "empty_as_zero")]
    pub ord_frozen: Decimal,
    #[serde(deserialize_with = "empty_as_zero")]
    pub liab: Decimal,
    #[serde(deserialize_with = "empty_as_zero")]
    pub upl: Decimal,
    #[serde(deserialize_with = "empty_as_zero")]
    pub upl_liab: Decimal,
    #[serde(deserialize_with = "empty_as_zero")]
    pub cross_liab: Decimal,
    #[serde(deserialize_with = "empty_as_zero")]
    pub iso_liab: Decimal,
    #[serde(deserialize_with = "empty_as_zero")]
    pub mgn_ratio: Decimal,
    #[serde(deserialize_with = "empty_as_zero")]
    pub interest: Decimal,
    #[serde(deserialize_with = "empty_as_zero")]
    pub twap: Decimal,
    #[serde(deserialize_with = "empty_as_zero")]
    pub max_loan: Decimal,
    #[serde(deserialize_with = "empty_as_zero")]
    pub eq_usd: Decimal,
    #[serde(deserialize_with = "empty_as_zero")]
    pub notional_lever: Decimal,
    #[serde(deserialize_with = "empty_as_zero")]
    pub stgy_eq: Decimal,
    #[serde(deserialize_with = "empty_as_zero")]
    pub iso_upl: Decimal,
    #[serde(deserialize_with = "empty_as_zero")]
    pub spot_in_use_amt: Decimal,
}

#[derive(Clone, Debug)]
pub struct OkxClient<S>(S);

impl<S> OkxClient<S> {
    pub fn new(x: S) -> Self {
        Self(x)
    }

    pub fn with_auth(self, authentication: Authentication) -> OkxPrivateClient<S> {
        OkxPrivateClient {
            client: self,
            authentication,
        }
    }
}

pub struct Authentication {
    key: String,
    secret: String,
    passphrase: String,
}

impl Authentication {
    pub fn new(key: String, secret: String, passphrase: String) -> Self {
        Self {
            key,
            secret,
            passphrase,
        }
    }
}

pub struct OkxPrivateClient<S> {
    client: OkxClient<S>,
    authentication: Authentication,
}

fn try_from_response<T>(
    x: hyper::Response<hyper::Body>,
) -> Pin<Box<dyn Future<Output = Result<T::Response, Error>> + Send + Sync + 'static>>
where
    T: Request,
    T::Response: DeserializeOwned,
{
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
            let resp: OkxResponse<T::Response> =
                serde_json::from_reader(hyper::body::Buf::reader(hyper::body::aggregate(x).await?))
                    .map_err(Error::DeserializeJsonBody)?;
            Ok(resp.data)
        })
    } else {
        Box::pin(async {
            let resp: OkxError =
                serde_json::from_reader(hyper::body::Buf::reader(hyper::body::aggregate(x).await?))
                    .map_err(Error::DeserializeJsonBody)?;
            Err(Error::RequestFailed {
                code: Some(resp.code),
                msg: Some(resp.msg),
            })
        })
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
        try_from_response::<T>(x)
    }
}

impl<T, S> Client<T> for OkxPrivateClient<S>
where
    T: Request + HttpRequest + Sealed + Signer + Serialize + Debug,
    T::Response: DeserializeOwned,
{
    type Service = S;

    type Error = Error;

    type TryFromResponseFuture =
        Pin<Box<dyn Future<Output = Result<T::Response, Self::Error>> + Send + Sync + 'static>>;

    fn service(&mut self) -> &mut Self::Service {
        &mut self.client.0
    }

    fn try_into_request(&mut self, x: T) -> Result<hyper::Request<hyper::Body>, Self::Error> {
        let query = serde_urlencoded::to_string(&x).map_err(Error::SerializeUrlencodedBody)?;
        let mut req = if x.method() == Method::GET {
            let mut req = hyper::Request::new(hyper::Body::empty());
            let uri = x.uri();
            assert_eq!(uri.query(), None);
            req.headers_mut()
                .append("Accept", "application/json".parse().unwrap());
            *req.uri_mut() = format!("{}?{}", uri, query).parse().unwrap();
            req
        } else {
            let mut req = hyper::Request::new(hyper::Body::from(query));
            let uri = x.uri();
            assert_eq!(uri.query(), None);
            req.headers_mut()
                .append("Accept", "application/json".parse().unwrap());
            *req.uri_mut() = uri;
            req
        };

        if <T::Signer as SignerKind>::is_private() {
            req.headers_mut()
                .insert("OK-ACCESS-KEY", self.authentication.key.parse().unwrap());
            let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
            req.headers_mut()
                .insert("OK-ACCESS-TIMESTAMP", timestamp.parse().unwrap());
            req.headers_mut().insert(
                "OK-ACCESS-PASSPHRASE",
                self.authentication.passphrase.parse().unwrap(),
            );
            let payload = dbg!(format!(
                "{}{}{}",
                timestamp,
                x.method(),
                x.uri().path_and_query().unwrap() // Schema always exists
            ));
            let mut mac = Hmac::<Sha256>::new_from_slice(self.authentication.secret.as_bytes())
                .expect("HMAC can take key of any size");
            mac.update(payload.as_bytes());
            let result = mac.finalize();
            req.headers_mut().insert(
                "OK-ACCESS-SIGN",
                BASE64_STANDARD.encode(result.into_bytes()).parse().unwrap(),
            );
        }

        Ok(req)
    }

    fn try_from_response(x: hyper::Response<hyper::Body>) -> Self::TryFromResponseFuture {
        try_from_response::<T>(x)
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

impl From<common::GetBalance> for GetV5AccountBalance {
    fn from(_: common::GetBalance) -> Self {
        Self { ccy: None }
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

impl<S> common::CommonOps for OkxPrivateClient<S> {
    type GetTickersRequest = GetV5MarketTickers;

    type GetTradesRequest = Unsupported;

    type GetOrderbookRequest = GetV5MarketBooks;

    type GetOrdersRequest = Unsupported;

    type GetAllOrdersRequest = Unsupported;

    type PlaceOrderRequest = Unsupported;

    type CancelOrderRequest = Unsupported;

    type CancelAllOrdersRequest = Unsupported;

    type GetBalanceRequest = GetV5AccountBalance;

    type GetPositionRequest = Unsupported;
}

impl<S> tower::Service<Unsupported> for OkxPrivateClient<S> {
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
mod __private {
    use crate::common::Unsupported;

    pub trait Sealed {}
    impl Sealed for Unsupported {}
}
