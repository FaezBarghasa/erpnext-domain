use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use crate::StockError;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StockQueueNode {
    pub qty: Decimal,
    pub rate: Decimal,
    pub timestamp: DateTime<Utc>,
}

/// Depletes quantity from FIFO queues and calculates COGS.
///
/// Algorithmic Complexity: $O(M \log M)$ to sort $M$ queue elements, and $O(M)$ depletion loop.
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

    #[test]
    fn test_fifo_depletion() {
        let t1 = Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap();
        let t2 = Utc.with_ymd_and_hms(2026, 6, 2, 12, 0, 0).unwrap();

        let mut queue = vec![
            StockQueueNode {
                qty: Decimal::new(10, 0),
                rate: Decimal::new(15, 0), // $15 each
                timestamp: t2,             // Newer
            },
            StockQueueNode {
                qty: Decimal::new(5, 0),
                rate: Decimal::new(10, 0), // $10 each
                timestamp: t1,             // Older
            },
        ];

        // Consume 8 items
        // First 5 should consume from t1 (5 * 10 = $50)
        // Next 3 should consume from t2 (3 * 15 = $45)
        // Total COGS = $95
        let cogs = consume_fifo_queue(&mut queue, Decimal::new(8, 0)).unwrap();
        assert_eq!(cogs, Decimal::new(95, 0));
        assert_eq!(queue.len(), 1);
        assert_eq!(queue[0].qty, Decimal::new(7, 0));
    }
}
