use hyper_tls::HttpsConnector;
use nerf::IntoService;
use nerf_exchanges::common::CommonOpsService;
use nerf_exchanges::upbit::UpbitClient;
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let mut svc = tower::ServiceBuilder::new()
        .layer_fn(|svc| {
            UpbitClient::new(svc)
                // .with_auth(KeySecretAuthentication::new(&key, &secret))
                .into_service()
        })
        .service(hyper::Client::builder().build(HttpsConnector::new()));

    let result = svc.get_orderbook("spot:BTC/KRW", None).await;
    info!(result = ?result);
}
