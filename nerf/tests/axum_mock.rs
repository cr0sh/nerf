#![cfg(test)]

use std::{error::Error, fmt::Debug, future::Future, io::Read, pin::Pin, sync::Arc};

use axum::{routing, Extension, Json};
use bytes::Buf;
use dashmap::DashMap;
use http::Method;
use http_body::Body;
use nerf::{Client, HttpRequest, IntoService, ReadyCall, Request};
use nerf_macros::{get, put};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio_stream::wrappers::ReceiverStream;
use tower::ServiceExt;
use tracing::{debug, trace};

use self::__private::Sealed;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct Item {
    name: String,
    score: f64,
    precedence: u32,
}

#[derive(Serialize, Deserialize, Debug)]
#[get("/api", response = GetItemsResponse)]
struct GetItems;

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
struct GetItemsResponse(Vec<Item>);

#[derive(Serialize, Deserialize, Debug)]
#[put("/api", response = PutItemResponse)]
#[serde(transparent)]
struct PutItem(Item);

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
struct PutItemResponse;

struct LocalTestClient<S> {
    inner: S,
}

impl<T, S> Client<T> for LocalTestClient<S>
where
    T: Request + HttpRequest + Sealed + Serialize + Debug,
    T::Response: DeserializeOwned,
{
    type Service = S;

    type Error = Box<dyn Error>;

    type TryFromResponseFuture = Pin<Box<dyn Future<Output = Result<T::Response, Self::Error>>>>;

    fn service(&mut self) -> &mut Self::Service {
        &mut self.inner
    }

    fn try_into_request(&mut self, x: T) -> Result<hyper::Request<hyper::Body>, Self::Error> {
        let uri = x.uri();
        if x.method() == Method::GET {
            Ok(hyper::Request::builder()
                .uri(format!("{uri}"))
                .method(x.method())
                .header("Content-Type", "application/json")
                .body(hyper::Body::empty())
                .unwrap())
        } else {
            Ok(hyper::Request::builder()
                .uri(format!("{uri}"))
                .method(x.method())
                .header("Content-Type", "application/json")
                .body(hyper::Body::from(serde_json::to_vec(&x).unwrap()))
                .unwrap())
        }
    }

    fn try_from_response(x: hyper::Response<hyper::Body>) -> Self::TryFromResponseFuture {
        debug!(status = ?x.status());
        Box::pin(async move {
            let mut s = String::new();
            hyper::body::aggregate(x)
                .await
                .unwrap()
                .reader()
                .read_to_string(&mut s)
                .unwrap();
            trace!(response_str = s);
            let resp = serde_json::from_str(&s).unwrap();
            Ok(resp)
        })
    }
}

fn create_service() -> impl tower::Service<
    hyper::Request<hyper::Body>,
    Response = hyper::Response<hyper::Body>,
    Error = hyper::Error,
> {
    let map = Arc::new(DashMap::<String, Item>::new());
    let router = axum::Router::new()
        .route("/api", routing::get(get_items).put(put_item))
        .layer(Extension(map));
    router
        .map_response(|mut x| {
            hyper::Response::new(hyper::Body::wrap_stream({
                let (tx, rx) = tokio::sync::mpsc::channel(1024);
                tokio::task::spawn(async move {
                    while let Some(b) = x.data().await {
                        tx.send(b).await.unwrap();
                    }
                });
                ReceiverStream::new(rx)
            }))
        })
        .map_err(|e| match e {})
        .boxed()
}

#[tracing::instrument(skip(map))]
async fn get_items(
    Extension(map): Extension<Arc<DashMap<String, Item>>>,
) -> Json<GetItemsResponse> {
    Json(GetItemsResponse(
        map.iter().map(|x| x.value().clone()).collect(),
    ))
}

#[tracing::instrument(skip(map))]
async fn put_item(
    Json(item): Json<Item>,
    Extension(map): Extension<Arc<DashMap<String, Item>>>,
) -> Json<PutItemResponse> {
    map.insert(item.name.clone(), item);
    Json(PutItemResponse)
}

#[tokio::test]
async fn test() {
    tracing_subscriber::fmt::init();

    let mut client = tower::ServiceBuilder::new()
        .layer_fn(|svc| (LocalTestClient { inner: svc }).into_service())
        .service(create_service());
    assert_eq!(client.ready_call(GetItems).await.unwrap().0.as_slice(), &[]);
    let item = Item {
        name: String::from("foo"),
        score: 4.2,
        precedence: 42,
    };
    assert_eq!(
        client.ready_call(PutItem(item.clone())).await.unwrap(),
        PutItemResponse
    );
    assert_eq!(
        client.ready_call(GetItems).await.unwrap().0.as_slice(),
        &[item]
    );
}

mod __private {
    pub trait Sealed {}
}
