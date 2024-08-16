use either::Either;
use ordered_float::OrderedFloat;
use rustc_hash::FxHashMap as HashMap;
use std::cell::Cell;
use std::cmp::min;
use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::rc::Rc;
use std::time::SystemTime;

static EPSILON: f64 = 1e-7;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Side {
    Bid,
    Ask,
}

// https://stackoverflow.com/a/32936064
thread_local!(static CLIENT_ID: Cell<u64> = Cell::new(0));

#[derive(Debug)]
pub struct Client {
    id: u64,
}

impl Client {
    pub fn new() -> Rc<Client> {
        CLIENT_ID.with(|thread_id| {
            let id = thread_id.get();
            thread_id.set(id + 1);
            Rc::new(Self { id })
        })
    }
}

impl fmt::Display for Client {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Client #{}", self.id)
    }
}

thread_local!(static ORDER_ID: Cell<u64> = Cell::new(0));

#[derive(Debug)]
pub struct Order {
    id: u64,
    side: Side,
    price: f64,
    size: u64,
    client: Rc<Client>,
    #[allow(dead_code)]
    timestamp: u128,
}

impl Order {
    pub fn new(side: Side, price: f64, size: u64, client: &Rc<Client>) -> Order {
        ORDER_ID.with(|thread_id| {
            let id = thread_id.get();
            thread_id.set(id + 1);
            Self {
                id,
                side,
                price,
                size,
                client: Rc::clone(client),
                timestamp: get_current_timestamp(),
            }
        })
    }
}

impl fmt::Display for Order {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}@{} {:?} order id {} from client id {}",
            self.size, self.price, self.side, self.id, self.client.id
        )
    }
}

type LadderLevel = VecDeque<Order>;
type Ladder = BTreeMap<OrderedFloat<f64>, LadderLevel>;

pub enum OrderBookResult {
    OrderId(u64),                   // passive placement
    Trades(Vec<Trade>),             // order matched
    OrderIdTrades(u64, Vec<Trade>), // order partially matched
    Error(&'static str),            // error
    Canceled,                       // order canceled
}

#[derive(Default, Debug)]
pub struct OrderBook {
    bids: Ladder,
    asks: Ladder,
    lookup: HashMap<u64, (Side, f64)>,
}

impl OrderBook {
    pub fn new() -> OrderBook {
        Self::default()
    }

    pub fn insert(&mut self, order: Order) -> OrderBookResult {
        if let Err(e) = self.validate_order(&order) {
            return OrderBookResult::Error(e);
        }

        if self.is_passive(&order) {
            OrderBookResult::OrderId(self.place_passive(order))
        } else {
            let mut order = Box::new(order);
            let trades = self.match_order(&mut order);

            match order.size {
                0 => OrderBookResult::Trades(trades.unwrap_or_default()),
                _ => {
                    let order_id = self.place_passive(*order);
                    OrderBookResult::OrderIdTrades(order_id, trades.unwrap_or_default())
                }
            }
        }
    }

    pub fn cancel(&mut self, order_id: u64) -> OrderBookResult {
        if let Some((side, price)) = self.lookup.remove(&order_id) {
            let ladder = self.get_ladder_mut(&side);
            let level = ladder.get_mut(&OrderedFloat(price)).unwrap();
            level.retain(|order| order.id != order_id);

            if level.is_empty() {
                ladder.remove(&OrderedFloat(price));
            }

            return OrderBookResult::Canceled;
        } else {
            return OrderBookResult::Error("Order does not exist");
        }
    }

    fn validate_order(&self, order: &Order) -> Result<(), &'static str> {
        if order.size > 0 && order.price > 0.0 {
            return Ok(());
        }
        Err("Non-positive price or quantity for an order")
    }

    fn place_passive(&mut self, order: Order) -> u64 {
        let order_id = order.id;
        self.lookup.insert(order_id, (order.side, order.price));
        let ladder = self.get_ladder_mut(&order.side);
        let price = OrderedFloat(order.price);

        match ladder.get_mut(&price) {
            Some(level) => {
                level.push_back(order);
            }
            _ => {
                ladder.insert(price, VecDeque::from(vec![order]));
            }
        };

        order_id
    }

    fn match_order(&mut self, order: &mut Order) -> Option<Vec<Trade>> {
        let mut empty_levels: Vec<OrderedFloat<f64>> = Vec::new();
        let mut trades: Vec<Trade> = Vec::new();
        let ladder = match order.side {
            Side::Bid => &mut self.asks,
            Side::Ask => &mut self.bids,
        };

        for (level_price, level) in match order.side {
            Side::Bid => Either::Left(ladder.iter_mut()),
            Side::Ask => Either::Right(ladder.iter_mut().rev()),
        } {
            let level_price = level_price.into_inner();

            if is_deeper(level_price, order.price, &order.side) {
                break;
            }

            for level_order in level.iter_mut() {
                if order.size == 0 {
                    break;
                }

                let trade_size = min(level_order.size, order.size);
                let trade = Trade::new(level_price, trade_size);
                level_order.size -= trade_size;
                order.size -= trade_size;
                trades.push(trade);
            }

            level.retain(|order| order.size > 0);

            if level.is_empty() {
                empty_levels.push(OrderedFloat(level_price));
            }
        }

        for level_price in empty_levels.iter() {
            ladder.remove(level_price);
        }

        match trades.is_empty() {
            false => Some(trades),
            true => None,
        }
    }

    fn get_size(&self, side: Side, price: f64) -> u64 {
        self.get_ladder(&side)
            .get(&OrderedFloat(price))
            .map_or(0, get_level_size)
    }

    fn get_ladder(&self, side: &Side) -> &Ladder {
        match side {
            Side::Bid => &self.bids,
            Side::Ask => &self.asks,
        }
    }

    fn get_ladder_mut(&mut self, side: &Side) -> &mut Ladder {
        match side {
            Side::Bid => &mut self.bids,
            Side::Ask => &mut self.asks,
        }
    }

    /// Best bid price
    pub fn best_bid(&self) -> Option<f64> {
        self.bids.keys().rev().next().map(|bid| bid.into_inner())
    }

    /// Volume of all orders at best bid price
    pub fn best_bid_size(&self) -> Option<u64> {
        self.bids.values().rev().next().map(get_level_size)
    }

    pub fn best_ask(&self) -> Option<f64> {
        self.asks.keys().next().map(|ask| ask.into_inner())
    }

    pub fn best_ask_size(&self) -> Option<u64> {
        self.asks.values().next().map(get_level_size)
    }

    fn is_passive(&self, order: &Order) -> bool {
        let best_bid = self.best_bid();
        let best_ask = self.best_ask();

        if (order.side == Side::Bid && best_ask.is_none())
            || (order.side == Side::Ask && best_bid.is_none())
        {
            true
        } else if order.side == Side::Bid {
            order.price < best_ask.unwrap() - EPSILON
        } else {
            order.price > best_bid.unwrap() + EPSILON
        }
    }
}

impl fmt::Display for OrderBook {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut msg: String = format!("Bid Qty   Price   Ask Qty\n");
        msg = format!("{}--------+-------+--------\n", msg);

        for ask in self.asks.keys().rev().take(5) {
            msg = format!(
                "{}           {:>2.2}   {:>5}\n",
                msg,
                ask,
                self.get_size(Side::Ask, ask.into_inner())
            );
        }

        for bid in self.bids.keys().rev().take(5) {
            msg = format!(
                "{}{:>7}    {:>2.2}\n",
                msg,
                self.get_size(Side::Bid, bid.into_inner()),
                bid
            );
        }

        write!(f, "{}", msg)
    }
}

thread_local!(static TRADE_ID: Cell<u64> = Cell::new(0));

pub struct Trade {
    id: u64,
    price: f64,
    size: u64,
    #[allow(dead_code)]
    timestamp: u128,
}

impl Trade {
    pub fn new(price: f64, size: u64) -> Trade {
        TRADE_ID.with(|thread_id| {
            let id = thread_id.get();
            thread_id.set(id + 1);
            Self {
                id,
                price,
                size,
                timestamp: get_current_timestamp(),
            }
        })
    }
}

impl fmt::Display for Trade {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Trade id {}, price {}, size {}",
            self.id, self.price, self.size
        )
    }
}

#[inline(always)]
fn get_current_timestamp() -> u128 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

#[inline]
fn get_level_size(level: &LadderLevel) -> u64 {
    level.iter().map(|order| order.size).sum()
}

/// check if
///  `a` price level is deeper in the book than `b`
#[inline(always)]
fn is_deeper(a: f64, b: f64, side: &Side) -> bool {
    match side {
        Side::Bid => a - EPSILON > b,
        Side::Ask => a + EPSILON < b,
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rstest::*;

    #[fixture]
    fn ob() -> OrderBook {
        OrderBook::new()
    }

    #[rstest]
    #[case(Side::Bid)]
    #[case(Side::Ask)]
    fn test_empty_order_book(#[by_ref] ob: &OrderBook, #[case] side: Side) {
        assert!(ob.get_ladder(&side).is_empty());
    }

    #[rstest]
    fn test_empty_bid(#[by_ref] ob: &OrderBook) {
        assert_eq!(ob.best_bid(), None);
        assert_eq!(ob.best_bid_size(), None);
    }

    #[rstest]
    fn test_empty_ask(#[by_ref] ob: &OrderBook) {
        assert_eq!(ob.best_ask(), None);
        assert_eq!(ob.best_ask_size(), None);
    }

    #[test]
    fn test_client_id() {
        let client1 = Client::new();
        let client2 = Client::new();
        assert_ne!(client1.id, client2.id);
    }

    #[fixture]
    fn client() -> Rc<Client> {
        Client::new()
    }

    #[rstest]
    #[case(-1.0, 0)]
    #[case(-1.0, 1)]
    #[case(0.0, 1)]
    fn test_invalid_order(
        #[by_ref] ob: &OrderBook,
        client: Rc<Client>,
        #[case] price: f64,
        #[case] size: u64,
    ) {
        let order = Order::new(Side::Bid, price, size, &client);
        assert!(ob.validate_order(&order).is_err());
    }

    #[fixture]
    fn order(client: Rc<Client>) -> Order {
        Order::new(Side::Bid, 1.0, 1, &client)
    }

    #[rstest]
    fn test_valid_order(#[by_ref] ob: &OrderBook, #[by_ref] order: &Order) {
        assert!(ob.validate_order(&order).is_ok());
    }

    #[rstest]
    fn test_passive_placement(mut ob: OrderBook, order: Order) {
        let result = ob.insert(order);
        assert!(matches!(result, OrderBookResult::OrderId(_)));
    }

    #[rstest]
    fn test_cancel_order(mut ob: OrderBook, order: Order) {
        let order_id = match ob.insert(order) {
            OrderBookResult::OrderId(id) => id,
            _ => unreachable!(),
        };
        let result = ob.cancel(order_id);
        assert!(matches!(result, OrderBookResult::Canceled));
    }

    #[rstest]
    fn test_cancel_invalid_order(mut ob: OrderBook, order: Order) {
        ob.insert(order);
        let result = ob.cancel(18378);
        assert!(matches!(result, OrderBookResult::Error(_)));
    }

    #[rstest]
    fn test_best_bid(mut ob: OrderBook, client: Rc<Client>) {
        let prices = vec![1.4, 1.5, 1.6, 1.3, 1.8, 1.4];
        let sizes = vec![1, 2, 3, 4, 5, 6];

        for (price, size) in prices.iter().zip(sizes.iter()) {
            let order = Order::new(Side::Bid, *price, *size, &client);
            ob.insert(order);
        }

        assert_eq!(ob.best_bid(), Some(1.8));
        assert_eq!(ob.best_bid_size(), Some(5));
    }

    #[rstest]
    fn test_best_ask(mut ob: OrderBook, client: Rc<Client>) {
        let prices = vec![1.4, 1.5, 1.6, 1.3, 1.8, 1.4];
        let sizes = vec![1, 2, 3, 4, 5, 6];

        for (price, size) in prices.iter().zip(sizes.iter()) {
            let order = Order::new(Side::Ask, *price, *size, &client);
            ob.insert(order);
        }

        assert_eq!(ob.best_ask(), Some(1.3));
        assert_eq!(ob.best_ask_size(), Some(4));
    }

    #[rstest]
    fn test_partial_fill(mut ob: OrderBook, client: Rc<Client>) {
        let order1 = Order::new(Side::Bid, 1.5, 1, &client);
        let order2 = Order::new(Side::Ask, 1.5, 2, &client);
        ob.insert(order1);

        if let OrderBookResult::OrderIdTrades(_, trades) = ob.insert(order2) {
            let trade = &trades[0];
            assert_eq!(trade.price, 1.5);
            assert_eq!(trade.size, 1);
            assert_eq!(trades.len(), 1);
        } else {
            unreachable!();
        }

        assert_eq!(ob.best_ask_size(), Some(1));
    }
}
