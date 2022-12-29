use rust_decimal::Decimal;

use nerf_exchanges::common::{Orderbook, OrderbookItem};

pub mod fetcher;

pub trait OrderbookExt {
    /// Calculate the base asset quantity if quote asset of `quote_quantity` used by market buy.
    fn taker_buy(&self, quote_quantity: Decimal) -> Result<Decimal, (Decimal, Decimal)>;
    /// Reverse backtracking of [`taker_buy`].
    ///
    /// [`taker_buy`]: OrderbookExt::taker_buy
    fn taker_buy_reversed(&self, base_quantity: Decimal) -> Result<Decimal, (Decimal, Decimal)>;
    /// Calculate the quote asset quantity if base asset of `base_quantity` used by market sell.
    fn taker_sell(&self, base_quantity: Decimal) -> Result<Decimal, (Decimal, Decimal)>;
    /// Reverse backtracking of [`taker_sell`].
    ///
    /// [`taker_sell`]: OrderbookExt::taker_sell
    fn taker_sell_reversed(&self, quote_quantity: Decimal) -> Result<Decimal, (Decimal, Decimal)>;
}

/// 'Consume's the orderbook units.
/// The second return value is the quantity of input asset which is not taken yet.
fn consume_by_base(units: &[OrderbookItem], mut base_qty: Decimal) -> (Decimal, Option<Decimal>) {
    let mut r = Decimal::ZERO;
    for unit in units {
        if base_qty <= Decimal::ZERO {
            return (r, None);
        }

        let to_commit = base_qty.min(unit.quantity);
        base_qty -= to_commit;
        r += unit.price * to_commit;
    }

    if base_qty > Decimal::ZERO {
        return (r, Some(base_qty));
    }
    (r, None)
}

/// 'Consume's the orderbook units by quote.
/// The second return value is the quantity of input asset which is not taken yet.
fn consume_by_quote(units: &[OrderbookItem], mut quote_qty: Decimal) -> (Decimal, Option<Decimal>) {
    let mut r = Decimal::ZERO;
    for unit in units {
        if quote_qty <= Decimal::ZERO {
            return (r, None);
        }

        if unit.price.is_zero() {
            break;
        }

        let to_commit = quote_qty.min(unit.price * unit.quantity);
        quote_qty -= to_commit;
        r += to_commit / unit.price;
    }

    if quote_qty > Decimal::ZERO {
        return (r, Some(quote_qty));
    }
    (r, None)
}

impl OrderbookExt for Orderbook {
    fn taker_buy(&self, quote_quantity: Decimal) -> Result<Decimal, (Decimal, Decimal)> {
        match consume_by_quote(self.asks(), quote_quantity) {
            (b, None) => Ok(b),
            (b, Some(r)) => Err((b, r)),
        }
    }

    fn taker_buy_reversed(&self, base_quantity: Decimal) -> Result<Decimal, (Decimal, Decimal)> {
        match consume_by_base(self.asks(), base_quantity) {
            (b, None) => Ok(b),
            (b, Some(r)) => Err((b, r)),
        }
    }

    fn taker_sell(&self, base_quantity: Decimal) -> Result<Decimal, (Decimal, Decimal)> {
        match consume_by_base(self.bids(), base_quantity) {
            (b, None) => Ok(b),
            (b, Some(r)) => Err((b, r)),
        }
    }

    fn taker_sell_reversed(&self, quote_quantity: Decimal) -> Result<Decimal, (Decimal, Decimal)> {
        match consume_by_quote(self.bids(), quote_quantity) {
            (b, None) => Ok(b),
            (b, Some(r)) => Err((b, r)),
        }
    }
}

#[cfg(test)]
mod tests {
    use nerf_exchanges::common::OrderbookItem;

    use crate::{consume_by_base, consume_by_quote};

    fn construct_units(x: Vec<(i64, i64)>) -> Vec<OrderbookItem> {
        x.into_iter()
            .map(|(price, qty)| OrderbookItem {
                price: price.try_into().unwrap(),
                quantity: qty.try_into().unwrap(),
            })
            .collect()
    }

    #[test]
    fn test_consume_by_base() {
        let units = construct_units(vec![(1, 3), (3, 2), (5, 3), (6, 3)]);

        assert_eq!(
            consume_by_base(&units, 10.try_into().unwrap()),
            ((3 + 6 + 15 + 12).try_into().unwrap(), None)
        );

        assert_eq!(
            consume_by_base(&units, 100.try_into().unwrap()),
            (
                (3 + 6 + 15 + 18).try_into().unwrap(),
                Some(89.try_into().unwrap())
            )
        );

        let units = construct_units(vec![(7, 3), (5, 2), (4, 3), (2, 3)]);

        assert_eq!(
            consume_by_base(&units, 10.try_into().unwrap()),
            ((21 + 10 + 12 + 4).try_into().unwrap(), None)
        );

        assert_eq!(
            consume_by_base(&units, 100.try_into().unwrap()),
            (
                (21 + 10 + 12 + 6).try_into().unwrap(),
                Some(89.try_into().unwrap())
            )
        );
    }

    #[test]
    fn test_consume_by_quote() {
        let units = construct_units(vec![(1, 3), (3, 2), (5, 3), (6, 3)]);

        assert_eq!(
            consume_by_quote(&units, (3 + 6 + 15 + 12).try_into().unwrap()),
            (10.try_into().unwrap(), None)
        );

        assert_eq!(
            consume_by_quote(&units, 100.try_into().unwrap()),
            (
                11.try_into().unwrap(),
                Some((100 - (3 + 6 + 15 + 18)).try_into().unwrap())
            )
        );

        let units = construct_units(vec![(7, 3), (5, 2), (4, 3), (2, 3)]);

        assert_eq!(
            consume_by_quote(&units, (21 + 10 + 12 + 4).try_into().unwrap()),
            (10.try_into().unwrap(), None)
        );

        assert_eq!(
            consume_by_quote(&units, 100.try_into().unwrap()),
            (
                11.try_into().unwrap(),
                Some((100 - (21 + 10 + 12 + 6)).try_into().unwrap())
            )
        );
    }
}
