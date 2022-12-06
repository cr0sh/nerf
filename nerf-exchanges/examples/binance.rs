use hyper_tls::HttpsConnector;
use nerf::{IntoService, ReadyCall};
use nerf_exchanges::{
    binance::{self, BinanceSpotClient},
    common::{CommonOpsService, Order, Side},
    KeySecretAuthentication,
};
use rust_decimal_macros::dec;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt::init();

    let key = std::env::var("BINANCE_API_KEY").unwrap();
    let secret = std::env::var("BINANCE_API_SECRET").unwrap();

    let mut svc = tower::ServiceBuilder::new()
        .layer_fn(|svc| {
            BinanceSpotClient::new(svc)
                .with_auth(KeySecretAuthentication::new(&key, &secret))
                .into_service()
        })
        .service(hyper::Client::builder().build(HttpsConnector::new()));

    let result = svc
        .ready_call(binance::GetApiV3Depth {
            symbol: "BTCBUSD".to_string(),
            limit: Some(100),
        })
        .await?;

    tracing::info!("Result: {:#?}", result);

    let result = svc.get_trades("swap:BTC/USDT").await?;

    tracing::info!("Result: {:#?}", result);

    let result = svc
        .place_order(
            "swap:BTC/USDT",
            Order::Market {
                side: Side::Buy,
                quantity: dec!(0.0001),
            },
            false,
        )
        .await?;

    tracing::info!("Result: {:#?}", result);

    Ok(())
}
