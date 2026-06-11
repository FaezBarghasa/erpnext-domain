use chrono::Utc;
use rust_decimal::Decimal;
use std::time::Instant;
use erp_inventory::stock_ledger::{consume_fifo_queue, StockQueueNode};

fn main() {
    println!("Running valuation_bench (consume_fifo_queue benchmark)...");

    // 1. Setup a queue with 10,000 nodes
    let num_nodes = 10_000;
    let mut fifo_queue = Vec::with_capacity(num_nodes);
    let now = Utc::now();
    for _ in 0..num_nodes {
        fifo_queue.push(StockQueueNode {
            qty: Decimal::new(10, 0),
            rate: Decimal::new(100, 0),
            timestamp: now,
        });
    }

    // 2. Measure the time to consume half of the queue (5,000 nodes -> 50,000 units)
    let issue_qty = Decimal::new(50_000, 0);

    let start = Instant::now();
    let res = consume_fifo_queue(&mut fifo_queue, issue_qty);
    let duration = start.elapsed();

    assert!(res.is_ok(), "FIFO consumption failed");
    let cogs = res.unwrap();
    assert_eq!(cogs, Decimal::new(5_000_000, 0)); // 50,000 * 100

    println!("Success! Consumed 5,000 nodes.");
    println!("Total duration: {:?}", duration);
    println!("Mean time per node: {:?}", duration / 5000);
}
