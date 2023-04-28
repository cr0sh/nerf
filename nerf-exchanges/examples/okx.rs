use hyper_tls::HttpsConnector;
use nerf::IntoService;
use nerf_exchanges::common::CommonOpsService;
use nerf_exchanges::okx::OkxClient;
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let key = std::env::var("OKX_API_KEY").unwrap();
    let secret = std::env::var("OKX_API_SECRET").unwrap();
    let passphrase = std::env::var("OKX_API_PASSPHRASE").unwrap();

    let mut svc = tower::ServiceBuilder::new()
        .layer_fn(|svc| {
            OkxClient::new(svc)
                .with_auth(nerf_exchanges::okx::Authentication::new(
                    key.clone(),
                    secret.clone(),
                    passphrase.clone(),
                ))
                .into_service()
        })
        .service(hyper::Client::builder().build(HttpsConnector::new()));

    let result = svc.get_balance().await;
    info!(result = ?result);
}
