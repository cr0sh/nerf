use crate::{
    common::{self, CommonOps, Disabled, IntoCommon, Private, Signer, SignerKind, Unsupported},
    Error, KeySecretAuthentication,
};

use chrono::{serde::ts_milliseconds, DateTime, Utc};
use hmac::{Hmac, Mac};
use http::{Method, StatusCode, Uri};
use hyper::body::Buf;
use jwt::SignWithKey;
use nerf::{delete, get, post, tag, Client, HttpRequest, Request};
use rust_decimal::Decimal;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_with::skip_serializing_none;
use sha2::{Digest, Sha256, Sha512};
use uuid::Uuid;

use std::convert::Infallible;
use std::fmt::{Debug, Write};
use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;

use self::__private::Sealed;

impl From<Infallible> for Error {
    fn from(x: Infallible) -> Self {
        match x {}
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    #[serde(rename = "bid")]
    Buy,
    #[serde(rename = "ask")]
    Sell,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    #[serde(rename = "limit")]
    Limit,
    #[serde(rename = "price")]
    MarketBuy,
    #[serde(rename = "market")]
    MarketSell,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderState {
    Wait,
    Watch,
    Done,
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortOrders {
    #[serde(rename = "asc")]
    Ascending,
    #[serde(rename = "desc")]
    Descending,
}

#[skip_serializing_none]
#[derive(Clone, Debug)]
#[get("https://api.upbit.com/v1/orderbook", response = GetV1OrderbookResponse)]
#[tag(Signer = Disabled)]
pub struct GetV1Orderbook {
    pub markets: Vec<String>,
}

impl Serialize for GetV1Orderbook {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct Projected {
            markets: String,
        }

        Projected {
            markets: self.markets.join(","),
        }
        .serialize(serializer)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct GetV1OrderbookResponse(pub Vec<GetV1OrderbookResponseItem>);

#[derive(Clone, Debug, Deserialize)]
pub struct GetV1OrderbookResponseItem {
    pub market: String,
    #[serde(with = "ts_milliseconds")]
    pub timestamp: DateTime<Utc>,
    pub total_ask_size: Decimal,
    pub total_bid_size: Decimal,
    pub orderbook_units: Vec<OrderbookUnit>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct OrderbookUnit {
    pub ask_price: Decimal,
    pub ask_size: Decimal,
    pub bid_price: Decimal,
    pub bid_size: Decimal,
}

#[derive(Clone, Debug, Serialize)]
#[get("https://api.upbit.com/v1/accounts", response = GetV1AccountsResponse)]
#[tag(Signer = Private)]
pub struct GetV1Accounts;

#[derive(Clone, Debug, Deserialize)]
pub struct GetV1AccountsResponse(pub Vec<GetV1AccountsResponseItem>);

#[derive(Clone, Debug, Deserialize)]
pub struct GetV1AccountsResponseItem {
    pub currency: String,
    pub balance: Decimal,
    pub locked: Decimal,
    pub avg_buy_price: Decimal,
    pub avg_buy_price_modified: bool,
    pub unit_currency: String,
}

#[skip_serializing_none]
#[derive(Clone, Debug, Serialize)]
#[post("https://api.upbit.com/v1/orders", response = PostV1OrdersResponse)]
#[tag(Signer = Private)]
pub struct PostV1Orders {
    pub market: String,
    pub side: Side,
    pub volume: Option<Decimal>,
    pub price: Option<Decimal>,
    pub ord_type: OrderType,
    pub identifier: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PostV1OrdersResponse {
    pub uuid: Uuid,
    pub side: Side,
    pub ord_type: OrderType,
    pub price: Option<Decimal>,
    pub avg_price: Option<Decimal>,
    pub state: OrderState,
    pub market: String,
    pub created_at: DateTime<Utc>,
    pub volume: Option<Decimal>,
    pub remaining_volume: Option<Decimal>,
    pub reserved_fee: Decimal,
    pub remaining_fee: Decimal,
    pub paid_fee: Decimal,
    pub locked: Decimal,
    pub executed_volume: Decimal,
    pub trades_count: u64,
}

#[skip_serializing_none]
#[derive(Clone, Debug, Serialize)]
#[get("https://api.upbit.com/v1/orders", response = GetV1OrdersResponse)]
#[tag(Signer = Private)]
pub struct GetV1Orders {
    pub market: String,
    pub uuids: Vec<String>,
    pub identifiers: Vec<String>,
    pub state: Option<OrderState>,
    pub states: Option<Vec<OrderState>>,
    pub page: Option<Decimal>,
    pub limit: Option<Decimal>,
    pub order_by: SortOrders,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GetV1OrdersResponse(pub Vec<GetV1OrdersResponseItem>);

#[derive(Clone, Debug, Deserialize)]
pub struct GetV1OrdersResponseItem {
    pub uuid: String,
    pub side: Side,
    pub ord_type: OrderType,
    pub price: Decimal,
    pub state: OrderState,
    pub market: String,
    pub created_at: DateTime<Utc>,
    pub volume: Decimal,
    pub remaining_volume: Decimal,
    pub reserved_fee: Decimal,
    pub remaining_fee: Decimal,
    pub paid_fee: Decimal,
    pub locked: Decimal,
    pub executed_volume: Decimal,
    pub trades_count: u64,
}

#[skip_serializing_none]
#[derive(Clone, Debug, Serialize)]
#[delete("https://api.upbit.com/v1/order", response = DeleteV1OrderResponse)]
#[tag(Signer = Private)]
pub struct DeleteV1Order {
    pub uuid: Option<String>,
    pub identifier: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DeleteV1OrderResponse {
    pub uuid: Uuid,
    pub side: Side,
    pub ord_type: OrderType,
    pub price: Option<Decimal>,
    pub state: String,
    pub market: String,
    pub created_at: DateTime<Utc>,
    pub volume: Option<Decimal>,
    pub remaining_volume: Option<Decimal>,
    pub reserved_fee: Decimal,
    pub remaining_fee: Decimal,
    pub paid_fee: Decimal,
    pub locked: Decimal,
    pub executed_volume: Decimal,
    pub trades_count: u64,
}

#[derive(Clone, Debug)]
pub struct UpbitClient<S>(S);

impl<S> UpbitClient<S> {
    pub fn new(x: S) -> Self {
        Self(x)
    }

    pub fn with_auth(self, authentication: KeySecretAuthentication) -> UpbitPrivateClient<S> {
        UpbitPrivateClient {
            client: self,
            authentication,
        }
    }
}

#[derive(Clone, Debug)]
pub struct UpbitPrivateClient<S> {
    client: UpbitClient<S>,
    authentication: KeySecretAuthentication,
}

impl<T, S> Client<T> for UpbitClient<S>
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
        let query = serde_urlencoded_upbit::to_string(&x)
            .map_err(Error::SerializeUrlencodedBodyUpbit)?
            .replace("%5B", "[")
            .replace("%5D", "]");
        if x.method() == Method::GET {
            let mut req = hyper::Request::new(hyper::Body::empty());
            let uri = x.uri();
            assert_eq!(uri.query(), None);
            req.headers_mut()
                .append("Accept", "application/json".parse().unwrap());
            *req.uri_mut() = Uri::from_str(&format!("{}?{}", uri, query)).unwrap();
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
        if x.status() == StatusCode::OK {
            Box::pin(async {
                let resp = serde_json::from_reader(hyper::body::aggregate(x).await?.reader())
                    .map_err(Error::DeserializeJsonBody)?;

                Ok(resp)
            })
        } else {
            #[derive(Deserialize)]
            struct UpbitError {
                error: UpbitErrorInner,
            }

            #[derive(Deserialize)]
            struct UpbitErrorInner {
                name: String,
                message: String,
            }

            Box::pin(async {
                let UpbitError { error } =
                    serde_json::from_reader(hyper::body::aggregate(x).await?.reader())
                        .map_err(Error::DeserializeJsonBody)?;

                Err(Error::RequestFailed {
                    code: Some(error.name),
                    msg: Some(error.message),
                })
            })
        }
    }
}

impl<T, S> Client<T> for UpbitPrivateClient<S>
where
    T: Request + HttpRequest + Sealed + Signer + Serialize + Debug,
    T::Response: DeserializeOwned,
    T::Signer: SignerKind,
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
        let token = if <T::Signer>::is_private() {
            #[skip_serializing_none]
            #[derive(Serialize)]
            struct AuthPayload {
                access_key: String,
                nonce: Uuid,
                query_hash: Option<String>,
                query_hash_alg: Option<&'static str>,
            }

            let key: Hmac<Sha256> = Hmac::new_from_slice(self.authentication.secret().as_bytes())
                .expect("upbit: cannot initialize authentication");

            let query_hash = if query.is_empty() {
                None
            } else {
                let mut hash = Sha512::new();
                hash.update(query.as_bytes());
                let result = hash.finalize();
                let mut hash = String::with_capacity(64);
                for &b in result.as_slice() {
                    write!(&mut hash, "{:02x}", b).expect("Writing hash to string failed");
                }

                Some(hash)
            };

            let payload = AuthPayload {
                access_key: self.authentication.key().to_string(),
                nonce: Uuid::new_v4(),
                query_hash_alg: query_hash.as_ref().map(|_| "SHA512"),
                query_hash,
            };

            let token = payload.sign_with_key(&key).map_err(Error::Jwt)?;
            Some(token)
        } else {
            None
        };
        if x.method() == Method::GET {
            let mut req = hyper::Request::new(hyper::Body::empty());
            let uri = x.uri();
            assert_eq!(uri.query(), None);
            *req.method_mut() = x.method();
            req.headers_mut()
                .append("Accept", "application/json".parse().unwrap());
            if let Some(token) = token {
                req.headers_mut()
                    .append("Authorization", format!("Bearer {token}").parse().unwrap());
            }
            *req.uri_mut() = Uri::from_str(&format!("{}?{}", uri, query)).unwrap();
            Ok(req)
        } else {
            let mut req = hyper::Request::new(hyper::Body::from(
                serde_json::to_string(&x).map_err(Error::SerializeJsonBody)?,
            ));
            *req.method_mut() = x.method();
            let uri = x.uri();
            assert_eq!(uri.query(), None);
            req.headers_mut()
                .append("Accept", "application/json".parse().unwrap());
            req.headers_mut()
                .append("Content-Type", "application/json".parse().unwrap());
            if let Some(token) = token {
                req.headers_mut()
                    .append("Authorization", format!("Bearer {token}").parse().unwrap());
            }
            *req.uri_mut() = uri;
            Ok(req)
        }
    }

    fn try_from_response(x: hyper::Response<hyper::Body>) -> Self::TryFromResponseFuture {
        tracing::debug!(status = ?x.status());
        if x.status().is_success() {
            Box::pin(async {
                serde_json::from_reader(hyper::body::aggregate(x).await?.reader())
                    .map_err(Error::DeserializeJsonBody)
            })
        } else {
            #[derive(Deserialize)]
            struct UpbitError {
                error: UpbitErrorInner,
            }

            #[derive(Deserialize)]
            struct UpbitErrorInner {
                name: String,
                message: String,
            }

            Box::pin(async {
                let UpbitError { error } =
                    serde_json::from_reader(hyper::body::aggregate(x).await?.reader())
                        .map_err(Error::DeserializeJsonBody)?;

                Err(Error::RequestFailed {
                    code: Some(error.name),
                    msg: Some(error.message),
                })
            })
        }
    }
}

impl<S> tower::Service<Unsupported> for UpbitClient<S> {
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

impl<S> tower::Service<Unsupported> for UpbitPrivateClient<S> {
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

impl From<common::GetOrderbook> for GetV1Orderbook {
    fn from(x: common::GetOrderbook) -> Self {
        Self {
            markets: vec![format!("{}-{}", x.market.quote(), x.market.base())],
        }
    }
}

impl From<common::GetBalance> for GetV1Accounts {
    fn from(_: common::GetBalance) -> Self {
        Self
    }
}

impl From<common::PlaceOrder> for PostV1Orders {
    fn from(x: common::PlaceOrder) -> Self {
        assert_eq!(*x.market.kind(), common::MarketKind::Spot);
        match x.order {
            common::Order::Market { side, quantity } => Self {
                market: format!("{}-{}", x.market.quote(), x.market.base()),
                side: match side {
                    common::Side::Buy => Side::Buy,
                    common::Side::Sell => Side::Sell,
                },
                volume: (side == common::Side::Sell).then_some(quantity),
                price: (side == common::Side::Buy).then_some(quantity),
                ord_type: match side {
                    common::Side::Buy => OrderType::MarketBuy,
                    common::Side::Sell => OrderType::MarketSell,
                },
                identifier: None,
            },
            common::Order::Limit {
                side,
                quantity,
                price,
                time_in_force,
            } => {
                assert_eq!(
                    time_in_force,
                    common::TimeInForce::GoodTilCancled,
                    "Upbit does not support TIFs"
                );
                Self {
                    market: format!("{}-{}", x.market.quote(), x.market.base()),
                    side: match side {
                        common::Side::Buy => Side::Buy,
                        common::Side::Sell => Side::Sell,
                    },
                    volume: Some(quantity),
                    price: Some(price),
                    ord_type: OrderType::Limit,
                    identifier: None,
                }
            }
            _ => todo!(),
        }
    }
}

impl From<common::GetOrders> for GetV1Orders {
    fn from(x: common::GetOrders) -> Self {
        Self {
            market: format!("{}-{}", x.market.quote(), x.market.base()),
            uuids: Vec::new(),
            identifiers: Vec::new(),
            state: Some(OrderState::Wait),
            states: None,
            page: None,
            limit: None,
            order_by: SortOrders::Descending,
        }
    }
}

impl From<common::CancelOrder> for DeleteV1Order {
    fn from(x: common::CancelOrder) -> Self {
        Self {
            uuid: Some(x.order_id),
            identifier: None,
        }
    }
}

impl IntoCommon for GetV1OrderbookResponse {
    type Output = common::Orderbook;

    fn into_common(self) -> Self::Output {
        let [this]: [GetV1OrderbookResponseItem; 1] = self
            .0
            .try_into()
            .expect("only single orderbook response can be converted into the common type");

        this.into_common()
    }
}

impl IntoCommon for GetV1OrderbookResponseItem {
    type Output = common::Orderbook;

    fn into_common(self) -> Self::Output {
        let (bids, asks) = self
            .orderbook_units
            .into_iter()
            .map(
                |OrderbookUnit {
                     ask_price,
                     ask_size,
                     bid_price,
                     bid_size,
                 }| {
                    (
                        common::OrderbookItem::new(bid_price, bid_size),
                        common::OrderbookItem::new(ask_price, ask_size),
                    )
                },
            )
            .unzip();

        common::Orderbook::new(bids, asks, Some(self.timestamp))
    }
}

impl<S> CommonOps for UpbitClient<S> {
    type GetTickersRequest = Unsupported;

    type GetTradesRequest = Unsupported;

    type GetOrderbookRequest = GetV1Orderbook;

    type GetOrdersRequest = Unsupported;

    type GetAllOrdersRequest = Unsupported;

    type PlaceOrderRequest = Unsupported;

    type CancelOrderRequest = Unsupported;

    type CancelAllOrdersRequest = Unsupported;

    type GetBalanceRequest = Unsupported;

    type GetPositionRequest = Unsupported;
}

impl<S> CommonOps for UpbitPrivateClient<S> {
    type GetTickersRequest = Unsupported;

    type GetTradesRequest = Unsupported;

    type GetOrderbookRequest = GetV1Orderbook;

    type GetOrdersRequest = GetV1Orders;

    type GetAllOrdersRequest = Unsupported;

    /// EXTREME CAUTION: Market buy orders on upbit takes quantity as *quote* quantity,
    /// not *base* quantity. This behavior is not common on other exchanges.
    type PlaceOrderRequest = PostV1Orders;

    type CancelOrderRequest = DeleteV1Order;

    type CancelAllOrdersRequest = Unsupported;

    type GetBalanceRequest = GetV1Accounts;

    type GetPositionRequest = Unsupported;
}

mod __private {
    use crate::common::Unsupported;

    pub trait Sealed {}
    impl Sealed for Unsupported {}
}
