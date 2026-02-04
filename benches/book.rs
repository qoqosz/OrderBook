#![feature(test)]

extern crate orderbook;
extern crate test;

use orderbook::book::{Client, Order, OrderBook};
use orderbook::utils::generate_orders;
use test::Bencher;

#[bench]
fn bench_insert_orders(b: &mut Bencher) {
    b.iter(|| {
        let client = Client::new();
        let mut ob = OrderBook::new();
        for (side, price, quantity) in generate_orders(1337, 1000) {
            let order = Order::new(side, price, quantity.into(), &client);
            ob.insert(order);
        }
        ob
    });
}
