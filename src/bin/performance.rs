extern crate orderbook;

use orderbook::book::{Client, Order, OrderBook};
use orderbook::utils::generate_orders;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let n_orders = args
        .get(1)
        .unwrap_or(&"10_000".to_string())
        .parse::<usize>()
        .unwrap();
    let seed = args
        .get(2)
        .unwrap_or(&"1337".to_string())
        .parse::<u64>()
        .unwrap();

    let orders_data = generate_orders(seed, n_orders);
    let client = Client::new();

    let start = Instant::now();
    let mut ob = OrderBook::new();
    for (side, price, quantity) in orders_data {
        let order = Order::new(side, price, quantity.into(), &client);
        ob.insert(order);
    }
    let duration = start.elapsed();

    let orders_left = ob.order_count();
    let orders_filled = n_orders - orders_left;

    println!("\nBenchmark Stats:");
    println!("Total orders: {}", n_orders);
    println!("Orders filled: {}", orders_filled);
    println!("Orders left: {}", orders_left);
    println!("Time taken: {:?}", duration);
    println!(
        "Throughput: {:.0} orders per second",
        n_orders as f64 / duration.as_secs_f64()
    );
}
