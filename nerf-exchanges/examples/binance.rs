use hyper_tls::HttpsConnector;
use nerf::HyperLayer;
use nerf_exchanges::binance;
use tower::Service;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt::init();

    let key = std::env::var("BINANCE_API_KEY").unwrap();
    let secret = std::env::var("BINANCE_API_SECRET").unwrap();

    let mut svc = tower::ServiceBuilder::new()
        .layer(binance::BinanceLayer::new())
        .layer(nerf_exchanges::HttpSignLayer::new(
            binance::Authentication::new(key, secret),
        ))
        .layer(HyperLayer::new())
        .service(hyper::Client::builder().build(HttpsConnector::new()));

    let result = svc.call(binance::GetApiV3Account {}).await?;

    tracing::info!("Result: {:?}", result);

    Ok(())
}
