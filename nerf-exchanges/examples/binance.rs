use hyper_tls::HttpsConnector;
use nerf::{IntoService, ReadyCall};
use nerf_exchanges::{
    binance::{self, BinanceClient},
    common::CommonOpsService,
    KeySecretAuthentication,
};
use rust_decimal::Decimal;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt::init();

    let key = std::env::var("BINANCE_API_KEY").unwrap();
    let secret = std::env::var("BINANCE_API_SECRET").unwrap();

    let mut svc = tower::ServiceBuilder::new()
        .layer_fn(|svc| {
            BinanceClient::new(svc)
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

    let result = svc
        .ready_call(binance::PostApiV3Order {
            symbol: "USDCBUSD".to_string(),
            side: binance::Side::Buy,
            order_type: binance::OrderType::Market,
            time_in_force: None,
            quantity: Some(Decimal::new(100, 0)),
            quote_order_qty: None,
            price: None,
            new_client_order_id: None,
            stop_price: None,
            trailing_delta: None,
            iceberg_qty: None,
            new_order_resp_type: None,
        })
        .await?;
    tracing::info!("Result: {:#?}", result);

    let result = svc.get_trades("spot:BTC/USDT").await?;

    tracing::info!("Result: {:#?}", result);

    Ok(())
}
