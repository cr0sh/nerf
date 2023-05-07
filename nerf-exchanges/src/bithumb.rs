use std::{collections::HashMap, fmt::Debug, future::Future, pin::Pin};

use crate::{
    common::{self, Disabled, Private, Signer, Unsupported},
    ts_milliseconds_str, Error,
};
use __private::Sealed;

use chrono::{DateTime, Utc};
use http::Method;
use nerf::{get, post, tag, Client, HttpRequest, Request};
use rust_decimal::Decimal;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderType {
    Bid,
    Ask,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    Pending,
    Completed,
    Cancel,
}

#[skip_serializing_none]
#[derive(Clone, Debug, Serialize)]
#[get("https://api.upbit.com/public/orderbook/{order_currency}_{payment_currency}", response = GetPublicOrderbookResponse)]
#[tag(Signer = Disabled)]
pub struct GetPublicOrderbook {
    #[serde(skip)]
    pub order_currency: String,
    #[serde(skip)]
    pub payment_currency: String,
    pub count: Option<u64>,
}

#[skip_serializing_none]
#[derive(Clone, Debug, Serialize)]
#[get("https://api.bithumb.com/public/orderbook/ALL_{payment_currency}", response = GetPublicOrderbookAllResponse)]
#[tag(Signer = Disabled)]
pub struct GetPublicOrderbookAll {
    #[serde(skip)]
    pub payment_currency: String,
    pub count: Option<u64>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GetPublicOrderbookResponse {
    pub order_currency: String,
    pub payment_currency: String,
    pub bids: Vec<GetPublicOrderbookResponseItem>,
    pub asks: Vec<GetPublicOrderbookResponseItem>,
    #[serde(with = "ts_milliseconds_str")]
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GetPublicOrderbookResponseItem {
    pub quantity: Decimal,
    pub price: Decimal,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GetPublicOrderbookAllResponse {
    pub payment_currency: String,
    #[serde(with = "ts_milliseconds_str")]
    pub timestamp: DateTime<Utc>,
    #[serde(flatten)]
    pub orderbooks: HashMap<String, GetPublicOrderbookAllResponseItem>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GetPublicOrderbookAllResponseItem {
    pub order_currency: String,
    pub bids: Vec<GetPublicOrderbookResponseItem>,
    pub asks: Vec<GetPublicOrderbookResponseItem>,
}

#[skip_serializing_none]
#[derive(Clone, Debug, Serialize)]
#[post("https://api.bithumb.com/info/orders", response = Vec<PostInfoOrdersResponseItem>)]
#[tag(Signer = Private)]
pub struct PostInfoOrders {
    pub order_id: Option<String>,
    #[serde(rename = "type")]
    pub order_type: Option<OrderType>,
    pub count: u64,
    // TODO
    // #[serde(with = "ts_milliseconds_str")]
    // pub after: DateTime<Utc>,
    pub order_currency: String,
    pub payment_currency: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PostInfoOrdersResponseItem {
    pub order_currency: String,
    pub payment_currency: String,
    pub order_id: String,
    #[serde(with = "ts_milliseconds_str")]
    pub order_date: DateTime<Utc>,
    #[serde(rename = "type")]
    pub order_type: OrderType,
    pub watch_price: Decimal,
    pub units: Decimal,
    pub units_remaining: Decimal,
    pub price: Decimal,
}

#[skip_serializing_none]
#[derive(Clone, Debug, Serialize)]
#[post("https://api.bithumb.com/info/order_detail", response = PostInfoOrderDetailResponse)]
#[tag(Signer = Private)]
pub struct PostInfoOrderDetail {
    order_id: String,
    order_currency: String,
    payment_currency: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PostInfoOrderDetailResponse {
    #[serde(with = "ts_milliseconds_str")]
    pub order_date: DateTime<Utc>,
    #[serde(rename = "type")]
    pub order_type: OrderType,
    pub order_status: OrderStatus,
    pub order_currency: String,
    pub payment_currency: String,
    pub watch_price: Option<Decimal>,
    pub order_price: Decimal,
    pub order_qty: Decimal,
    #[serde(with = "ts_milliseconds_str")]
    pub cancel_date: DateTime<Utc>,
    pub cancel_type: String,
    // TODO
    pub contract: Vec<serde_json::Value>,
}

#[skip_serializing_none]
#[derive(Clone, Debug, Serialize)]
#[post("https://api.bithumb.com/trade/{place_or_market}", response = PostTradeResponse)]
#[tag(Signer = Private)]
pub struct PostTrade {
    pub place_or_market: String, // place, market_buy, market_sell
    pub order_currency: String,
    pub payment_currency: String,
    pub units: Decimal,
    pub price: Option<Decimal>,
    #[serde(rename = "type")]
    pub order_type: Option<OrderType>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PostTradeResponse {
    pub order_id: String,
}

#[skip_serializing_none]
#[derive(Clone, Debug, Serialize)]
#[post("https://api.bithumb.com/trade/cancel", response = ())]
#[tag(Signer = Private)]
pub struct PostTradeCancel {
    #[serde(rename = "type")]
    pub order_type: OrderType,
    pub order_id: String,
    pub order_currency: String,
    pub payment_currency: String,
}

#[derive(Clone, Debug)]
pub struct BithumbClient<S>(S);

impl<S> BithumbClient<S> {
    pub fn new(x: S) -> Self {
        Self(x)
    }
}

impl<T, S> Client<T> for BithumbClient<S>
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
        struct BithumbResponse<T> {
            #[allow(dead_code)]
            status: String,
            data: T,
        }

        #[derive(Debug, Deserialize)]
        struct BithumbError {
            status: String,
            message: String,
        }

        if x.status().is_success() {
            Box::pin(async {
                let buf = hyper::body::to_bytes(x).await?;
                let resp: BithumbResponse<T::Response> =
                    serde_json::from_slice(&buf).map_err(|e| {
                        Error::DeserializeJsonBody(e, String::from_utf8_lossy(&buf).to_string())
                    })?;
                Ok(resp.data)
            })
        } else {
            Box::pin(async {
                let buf = hyper::body::to_bytes(x).await?;
                let resp: BithumbError = serde_json::from_slice(&buf).map_err(|e| {
                    Error::DeserializeJsonBody(e, String::from_utf8_lossy(&buf).to_string())
                })?;
                Err(Error::RequestFailed {
                    code: Some(resp.status),
                    msg: Some(resp.message),
                })
            })
        }
    }
}

impl<S> tower::Service<Unsupported> for BithumbClient<S> {
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

impl From<common::GetOrderbook> for GetPublicOrderbook {
    fn from(x: common::GetOrderbook) -> Self {
        Self {
            order_currency: x.market.base().to_string(),
            payment_currency: x.market.quote().to_string(),
            count: x.ticks,
        }
    }
}

impl From<common::GetOrders> for PostInfoOrders {
    fn from(x: common::GetOrders) -> Self {
        Self {
            order_id: None,
            order_type: None,
            count: 100,
            order_currency: x.market.base().to_string(),
            payment_currency: x.market.quote().to_string(),
        }
    }
}

impl From<common::PlaceOrder> for PostTrade {
    fn from(x: common::PlaceOrder) -> Self {
        let order_currency = x.market.base().to_string();
        let payment_currency = x.market.quote().to_string();
        match x.order {
            common::Order::Market { side, quantity } => Self {
                place_or_market: if side == common::Side::Buy {
                    String::from("market_buy")
                } else {
                    String::from("market_sell")
                },
                order_currency,
                payment_currency,
                units: quantity,
                price: None,
                order_type: None,
            },
            common::Order::Limit {
                side,
                quantity,
                price,
                time_in_force: _,
            } => Self {
                place_or_market: String::from("place"),
                order_currency,
                payment_currency,
                units: quantity,
                price: Some(price),
                order_type: Some(if side == common::Side::Buy {
                    OrderType::Bid
                } else {
                    OrderType::Ask
                }),
            },
            _ => todo!(),
        }
    }
}

impl common::IntoCommon for GetPublicOrderbookResponse {
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
            Some(self.timestamp),
        )
    }
}

impl common::IntoCommon for GetPublicOrderbookAllResponseItem {
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
            None,
        )
    }
}

impl<S> common::CommonOps for BithumbClient<S> {
    type GetTickersRequest = Unsupported;

    type GetTradesRequest = Unsupported;

    type GetOrderbookRequest = GetPublicOrderbook;

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
