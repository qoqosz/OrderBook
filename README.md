# OrderBook

Simple Limit Order Book implementation in Rust

```rust
Initial order book
==================
Bid Qty   Price   Ask Qty
--------+-------+--------
           1.30       6
           1.20       2
           1.10       5
      3    1.00
      5    0.90

Trade id 0, price 1.1, size 2

After the trade
===============
Bid Qty   Price   Ask Qty
--------+-------+--------
           1.30       6
           1.20       2
           1.10       3
      3    1.00
      5    0.90

Placing order: <10@0.8 Bid order id 7 from client id 0>

New order book
==============
Bid Qty   Price   Ask Qty
--------+-------+--------
           1.30       6
           1.20       2
           1.10       3
      3    1.00
      5    0.90
     10    0.80

Canceling order: <7>
Order canceled
Order book back to previous state
==================================
Bid Qty   Price   Ask Qty
--------+-------+--------
           1.30       6
           1.20       2
           1.10       3
      3    1.00
      5    0.90

Taking all the liquidity on the ask side
Trade id 1, price 1.1, size 1
Trade id 2, price 1.1, size 2
Trade id 3, price 1.2, size 2
Trade id 4, price 1.3, size 6
Order placed: <8>
After the trade
===============
Bid Qty   Price   Ask Qty
--------+-------+--------
      9    1.40
      3    1.00
      5    0.90
```
