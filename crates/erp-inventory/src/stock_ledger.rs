use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use crate::StockError;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StockQueueNode {
    pub qty: Decimal,
    pub rate: Decimal,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ValuationMethod {
    FIFO,
    MovingAverage,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WarehouseStockState {
    pub valuation_method: ValuationMethod,
    pub qty: Decimal,
    pub moving_average_rate: Decimal,
    pub fifo_queue: Vec<StockQueueNode>,
}

impl WarehouseStockState {
    pub fn new(valuation_method: ValuationMethod) -> Self {
        Self {
            valuation_method,
            qty: Decimal::ZERO,
            moving_average_rate: Decimal::ZERO,
            fifo_queue: Vec::new(),
        }
    }
}

pub struct WarehouseStockRegistry {
    // Thread-safe map storing stock states for each (item_id, warehouse)
    pub states: RwLock<HashMap<(String, String), WarehouseStockState>>,
    pub allow_negative_stock: bool,
}

impl WarehouseStockRegistry {
    pub fn new(allow_negative_stock: bool) -> Self {
        Self {
            states: RwLock::new(HashMap::new()),
            allow_negative_stock,
        }
    }

    /// Records stock change and returns the computed COGS.
    ///
    /// Algorithmic Complexity: $O(M)$ where $M$ is queue length for FIFO issues.
    pub fn update_stock(
        &self,
        item_id: &str,
        warehouse: &str,
        qty_change: Decimal,
        rate: Decimal,
        timestamp: DateTime<Utc>,
    ) -> Result<Decimal, StockError> {
        let key = (item_id.to_string(), warehouse.to_string());
        
        // Write lock for modifying the state
        let mut states_guard = self.states.write().unwrap();
        let state = states_guard
            .entry(key)
            .or_insert_with(|| WarehouseStockState::new(ValuationMethod::MovingAverage));

        let new_qty = state.qty + qty_change;

        // Verify negative stock condition
        if new_qty.is_sign_negative() && !self.allow_negative_stock {
            return Err(StockError::InsufficientStock);
        }

        let mut cogs = Decimal::ZERO;

        if qty_change.is_sign_positive() {
            match state.valuation_method {
                ValuationMethod::MovingAverage => {
                    if new_qty.is_zero() {
                        state.moving_average_rate = Decimal::ZERO;
                    } else {
                        state.moving_average_rate = ((state.qty * state.moving_average_rate)
                            + (qty_change * rate))
                            / new_qty;
                    }
                }
                ValuationMethod::FIFO => {
                    state.fifo_queue.push(StockQueueNode {
                        qty: qty_change,
                        rate,
                        timestamp,
                    });
                }
            }
            state.qty = new_qty;
        } else if qty_change.is_sign_negative() {
            let issue_qty = -qty_change;
            match state.valuation_method {
                ValuationMethod::MovingAverage => {
                    cogs = issue_qty * state.moving_average_rate;
                    state.qty = new_qty;
                }
                ValuationMethod::FIFO => {
                    cogs = consume_fifo_queue(&mut state.fifo_queue, issue_qty)?;
                    state.qty = new_qty;
                }
            }
        }

        Ok(cogs)
    }

    /// Sets the valuation method for a specific item at a warehouse.
    pub fn set_valuation_method(&self, item_id: &str, warehouse: &str, method: ValuationMethod) {
        let key = (item_id.to_string(), warehouse.to_string());
        let mut states_guard = self.states.write().unwrap();
        let state = states_guard
            .entry(key)
            .or_insert_with(|| WarehouseStockState::new(method));
        state.valuation_method = method;
    }
}

/// Depletes quantity from FIFO queues and calculates COGS.
pub fn consume_fifo_queue(
    queue: &mut Vec<StockQueueNode>,
    mut issue_qty: Decimal,
) -> Result<Decimal, StockError> {
    if issue_qty.is_sign_negative() || issue_qty.is_zero() {
        return Ok(Decimal::ZERO);
    }

    // Ensure FIFO by sorting on timestamp
    queue.sort_by_key(|n| n.timestamp);

    let total_available: Decimal = queue.iter().map(|n| n.qty).sum();
    if total_available < issue_qty {
        return Err(StockError::InsufficientStock);
    }

    let mut cogs = Decimal::ZERO;

    while issue_qty > Decimal::ZERO && !queue.is_empty() {
        let node = &mut queue[0];
        if node.qty <= issue_qty {
            cogs += node.qty * node.rate;
            issue_qty -= node.qty;
            queue.remove(0);
        } else {
            cogs += issue_qty * node.rate;
            node.qty -= issue_qty;
            issue_qty = Decimal::ZERO;
        }
    }

    Ok(cogs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_fifo_depletion() {
        let t1 = Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap();
        let t2 = Utc.with_ymd_and_hms(2026, 6, 2, 12, 0, 0).unwrap();

        let mut queue = vec![
            StockQueueNode {
                qty: Decimal::new(10, 0),
                rate: Decimal::new(15, 0), // $15 each
                timestamp: t2,  // Newer
            },
            StockQueueNode {
                qty: Decimal::new(5, 0),
                rate: Decimal::new(10, 0), // $10 each
                timestamp: t1,  // Older
            },
        ];

        // Consume 8 items: 5 from t1 (5 * 10 = $50), 3 from t2 (3 * 15 = $45). Total = 95
        let cogs = consume_fifo_queue(&mut queue, Decimal::new(8, 0)).unwrap();
        assert_eq!(cogs, Decimal::new(95, 0));
        assert_eq!(queue.len(), 1);
        assert_eq!(queue[0].qty, Decimal::new(7, 0));
    }

    #[test]
    fn test_moving_average_valuation() {
        let registry = WarehouseStockRegistry::new(false);
        let now = Utc::now();

        // 1. Receive 10 units at $10 each
        let cogs1 = registry.update_stock("item_x", "wh_a", Decimal::new(10, 0), Decimal::new(10, 0), now).unwrap();
        assert_eq!(cogs1, Decimal::new(0, 0));
        
        // 2. Receive 5 units at $16 each
        let cogs2 = registry.update_stock("item_x", "wh_a", Decimal::new(5, 0), Decimal::new(16, 0), now).unwrap();
        assert_eq!(cogs2, Decimal::new(0, 0));

        // Average rate should be: (10 * 10 + 5 * 16) / 15 = (100 + 80) / 15 = 12
        let states = registry.states.read().unwrap();
        let state = states.get(&("item_x".to_string(), "wh_a".to_string())).unwrap();
        assert_eq!(state.moving_average_rate, Decimal::new(12, 0));
        assert_eq!(state.qty, Decimal::new(15, 0));
        drop(states);

        // 3. Issue 8 units, should cost 8 * 12 = 96
        let cogs3 = registry.update_stock("item_x", "wh_a", Decimal::new(-8, 0), Decimal::new(0, 0), now).unwrap();
        assert_eq!(cogs3, Decimal::new(96, 0));

        let states = registry.states.read().unwrap();
        let state = states.get(&("item_x".to_string(), "wh_a".to_string())).unwrap();
        assert_eq!(state.qty, Decimal::new(7, 0));
        assert_eq!(state.moving_average_rate, Decimal::new(12, 0));
    }

    #[test]
    fn test_concurrent_negative_stock_validation() {
        // Enforce no negative stock
        let registry = Arc::new(WarehouseStockRegistry::new(false));
        let now = Utc::now();

        // Put 10 units in stock
        registry.update_stock("item_y", "wh_a", Decimal::new(10, 0), Decimal::new(10, 0), now).unwrap();

        // Launch 10 threads trying to issue 2 units each concurrently.
        // Exactly 5 threads should succeed. The other 5 must fail with InsufficientStock.
        let mut handles = vec![];
        for _ in 0..10 {
            let reg_clone = Arc::clone(&registry);
            handles.push(thread::spawn(move || {
                reg_clone.update_stock("item_y", "wh_a", Decimal::new(-2, 0), Decimal::new(0, 0), now)
            }));
        }

        let mut successes = 0;
        let mut failures = 0;

        for h in handles {
            match h.join().unwrap() {
                Ok(_) => successes += 1,
                Err(StockError::InsufficientStock) => failures += 1,
                Err(_) => {}
            }
        }

        assert_eq!(successes, 5);
        assert_eq!(failures, 5);

        let states = registry.states.read().unwrap();
        let state = states.get(&("item_y".to_string(), "wh_a".to_string())).unwrap();
        assert_eq!(state.qty, Decimal::new(0, 0));
    }
}
