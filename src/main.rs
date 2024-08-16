use orderbook::book::{Client, Order, OrderBook, OrderBookResult, Side};

fn main() {
    let mut ob = OrderBook::new();
    let client1 = Client::new();
    let client2 = Client::new();

    // Initial order book
    {
        let orders = vec![
            Order::new(Side::Bid, 0.9, 5, &client1),
            Order::new(Side::Bid, 1.0, 3, &client1),
            Order::new(Side::Ask, 1.1, 3, &client1),
            Order::new(Side::Ask, 1.2, 2, &client1),
            Order::new(Side::Ask, 1.1, 2, &client2),
            Order::new(Side::Ask, 1.3, 6, &client2),
        ];

        for order in orders.into_iter() {
            ob.insert(order);
        }
    }

    println!("Initial order book\n==================\n{}", ob);

    // Placing a new order that will match the opposite side
    let mut order = Order::new(Side::Bid, 1.1, 2, &client2);

    match ob.insert(order) {
        OrderBookResult::Trades(trades) => {
            trades.iter().for_each(|trade| println!("{}", trade));
        }
        _ => println!("Sth went wrong"),
    };

    println!("After the trade\n===============\n{}", ob);

    // Placing a very passive order and then cancelling it
    order = Order::new(Side::Bid, 0.8, 10, &client1);
    println!("Placing order: <{}>", order);

    if let OrderBookResult::OrderId(order_id) = ob.insert(order) {
        println!("\nNew order book\n==============\n{}", ob);
        println!("Canceling order: <{}>", order_id);

        match ob.cancel(order_id) {
            OrderBookResult::Canceled => println!("Order canceled"),
            _ => println!("Order could not be canceled"),
        };
    }
    println!(
        "Order book back to previous state\n=================================\n{}",
        ob
    );

    // Order that takes all the liquidity
    println!("Taking all the liquidity on the ask side");
    order = Order::new(Side::Bid, 1.4, 20, &client2);

    match ob.insert(order) {
        OrderBookResult::OrderIdTrades(order_id, trades) => {
            trades.iter().for_each(|trade| println!("{}", trade));
            println!("Order placed: <{}>", order_id);
        }
        _ => println!("Sth went wrong"),
    };

    println!("After the trade\n===============\n{}", ob);
}
