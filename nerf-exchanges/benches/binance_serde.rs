use criterion::{black_box, criterion_group, criterion_main, Criterion};
use nerf_exchanges::binance;

const ORDERBOOK_STR: &str = include_str!("binance_sample_orderbook_100.json");

pub fn benchmark(c: &mut Criterion) {
    c.bench_function("deserialize orderbook(tick=100)", |b| {
        b.iter(|| parse_orderbook(black_box(ORDERBOOK_STR)))
    });
}

fn parse_orderbook(input: &str) {
    let _: binance::GetApiV3DepthResponse = serde_json::from_str(input).unwrap();
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
