use hyper_tls::HttpsConnector;
use nerf::HyperLayer;
use nerf_exchanges::binance;
use tower::Service;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt().init();

    let mut svc = tower::ServiceBuilder::new()
        .layer(binance::BinanceLayer::new())
        .layer(HyperLayer::new())
        .service(hyper::Client::builder().build(HttpsConnector::new()));

    let result = svc
        .call(binance::GetApiV3Trades {
            symbol: "BTCBUSD".into(),
            limit: None,
        })
        .await?;

    tracing::info!("Result: {:?}", result);

    Ok(())
}
