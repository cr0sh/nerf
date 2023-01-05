use std::{collections::HashMap, fmt::Debug, future::Future, pin::Pin};

use crate::{
    common::{self, Disabled, Signer, Unsupported},
    ts_milliseconds_str, Error,
};
use __private::Sealed;

use chrono::{DateTime, Utc};
use http::Method;
use nerf::{tag, Client, HttpRequest, Request};
use rust_decimal::Decimal;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Clone, Debug, Serialize)]
#[tag(Signer = Disabled)]
pub struct GetPublicOrderbook {
    #[serde(skip)]
    pub order_currency: String,
    #[serde(skip)]
    pub payment_currency: String,
    pub count: Option<u64>,
}

impl Request for GetPublicOrderbook {
    type Response = GetPublicOrderbookResponse;
}

impl HttpRequest for GetPublicOrderbook {
    fn uri(&self) -> http::Uri {
        format!(
            "https://api.bithumb.com/public/orderbook/{}_{}",
            self.order_currency, self.payment_currency
        )
        .parse()
        .expect("cannot parse the generated uri")
    }

    fn method(&self) -> http::Method {
        http::Method::GET
    }
}

impl Sealed for GetPublicOrderbook {}

#[skip_serializing_none]
#[derive(Clone, Debug, Serialize)]
#[tag(Signer = Disabled)]
pub struct GetPublicOrderbookAll {
    #[serde(skip)]
    pub payment_currency: String,
    pub count: Option<u64>,
}

impl Request for GetPublicOrderbookAll {
    type Response = GetPublicOrderbookAllResponse;
}

impl HttpRequest for GetPublicOrderbookAll {
    fn uri(&self) -> http::Uri {
        format!(
            "https://api.bithumb.com/public/orderbook/ALL_{}",
            self.payment_currency
        )
        .parse()
        .expect("cannot parse the generated uri")
    }

    fn method(&self) -> http::Method {
        http::Method::GET
    }
}

impl Sealed for GetPublicOrderbookAll {}

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
    quantity: Decimal,
    price: Decimal,
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
                let resp: BithumbResponse<T::Response> = serde_json::from_reader(
                    hyper::body::Buf::reader(hyper::body::aggregate(x).await?),
                )
                .map_err(Error::DeserializeJsonBody)?;
                Ok(resp.data)
            })
        } else {
            Box::pin(async {
                let resp: BithumbError = serde_json::from_reader(hyper::body::Buf::reader(
                    hyper::body::aggregate(x).await?,
                ))
                .map_err(Error::DeserializeJsonBody)?;
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
