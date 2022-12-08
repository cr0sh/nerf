use rust_decimal::Decimal;

/// Conversion into common types.
pub trait IntoCommon {
    type Output;
    fn into_common(self) -> Self::Output;
}

#[derive(Clone, Debug)]
pub struct Ticker {
    bid_price: Decimal,
    ask_price: Decimal,
}

impl Ticker {
    pub fn new(bid_price: Decimal, ask_price: Decimal) -> Self {
        Self {
            bid_price,
            ask_price,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Orderbook {
    bids: Vec<OrderbookItem>,
    asks: Vec<OrderbookItem>,
}

impl Orderbook {
    pub fn new(bids: Vec<OrderbookItem>, asks: Vec<OrderbookItem>) -> Self {
        Self { bids, asks }
    }

    pub fn bids(&self) -> &[OrderbookItem] {
        &self.bids
    }
    pub fn asks(&self) -> &[OrderbookItem] {
        &self.asks
    }
}

#[derive(Clone, Copy, Debug)]
pub struct OrderbookItem {
    pub price: Decimal,
    pub quantity: Decimal,
}

impl OrderbookItem {
    pub fn new(price: Decimal, quantity: Decimal) -> Self {
        Self { price, quantity }
    }
}
