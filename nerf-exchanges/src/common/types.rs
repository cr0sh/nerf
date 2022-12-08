use rust_decimal::Decimal;

/// Conversion into common types.
pub trait IntoCommon {
    type Output;
    fn into_common(self) -> Self::Output;
}

#[derive(Clone, Debug)]
pub struct Ticker {
    bid_price: Decimal,
    bid_quantity: Decimal,
    ask_price: Decimal,
    ask_quantity: Decimal,
}

impl Ticker {
    pub fn new(
        bid_price: Decimal,
        bid_quantity: Decimal,
        ask_price: Decimal,
        ask_quantity: Decimal,
    ) -> Self {
        Self {
            bid_price,
            bid_quantity,
            ask_price,
            ask_quantity,
        }
    }
}

impl From<Ticker> for Orderbook {
    fn from(x: Ticker) -> Self {
        Self {
            bids: vec![OrderbookItem {
                price: x.bid_price,
                quantity: x.bid_quantity,
            }],
            asks: vec![OrderbookItem {
                price: x.ask_price,
                quantity: x.ask_quantity,
            }],
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
