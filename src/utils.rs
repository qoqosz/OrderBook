use crate::book::Side;
use rand::{rngs::StdRng, Rng, SeedableRng};
use rand_distr::Normal;

pub const N_ORDERS: usize = 1_000;
pub const INITIAL_PRICE: f64 = 100.0;
pub const MU: f64 = 0.000005; // drift
pub const SIGMA: f64 = 0.005; // volatility

/// Generate order data following arithmetic Brownian motion.
/// Returns a vector of (side, price, quantity) tuples.
pub fn generate_orders(seed: u64, n: usize) -> Vec<(Side, f64, u32)> {
    let mut rng = StdRng::seed_from_u64(seed);
    let normal = Normal::new(0.0, 1.0).unwrap(); // standard normal for dW

    let mut price = INITIAL_PRICE;
    let mut orders = Vec::with_capacity(n);

    for _ in 0..n {
        // Arithmetic Brownian motion: dP = mu + sigma * dW
        let dw = rng.sample(normal);
        let dp = MU + SIGMA * dw;
        price = 0.01_f64.max(price + dp);
        price = (price * 100.0).round() / 100.0; // round to 0.01

        // Random side (bid or ask)
        let side = if rng.random_bool(0.5) {
            Side::Bid
        } else {
            Side::Ask
        };

        // Random quantity between 1 and 100
        let quantity: u32 = rng.random_range(1..=100);

        orders.push((side, price, quantity));
    }

    orders
}
