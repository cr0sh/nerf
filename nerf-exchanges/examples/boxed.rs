use anyhow::anyhow;
use hyper_tls::HttpsConnector;
use nerf::IntoService;
use nerf_exchanges::{
    binance::BinanceSpotClient,
    common::{BoxCommonOpsService, Order, Side},
    KeySecretAuthentication,
};
use rust_decimal_macros::dec;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt::init();

    let key = std::env::var("BINANCE_API_KEY").unwrap();
    let secret = std::env::var("BINANCE_API_SECRET").unwrap();

    let svc = tower::ServiceBuilder::new()
        .layer_fn(|svc| {
            BinanceSpotClient::new(svc)
                .with_auth(KeySecretAuthentication::new(&key, &secret))
                .into_service()
        })
        .service(hyper::Client::builder().build(HttpsConnector::new()));

    let mut svc = BoxCommonOpsService::new(svc);

    let result = svc
        .place_order(
            "swap:BTC/USDT",
            Order::Market {
                side: Side::Buy,
                quantity: dec!(0.0001),
            },
            false,
        )
        .await
        .map_err(|e| anyhow!(e))?;

    tracing::info!("Result: {:#?}", result);

    Ok(())
}
