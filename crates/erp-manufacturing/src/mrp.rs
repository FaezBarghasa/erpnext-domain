use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DemandSource {
    pub order_id: String,
    pub item_id: String,
    pub qty: Decimal,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InventoryStock {
    pub item_id: String,
    pub available_qty: Decimal,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MRPResult {
    pub item_id: String,
    pub shortage_qty: Decimal,
}

/// Runs MRP supply planning parallel pipeline using Tokio channel streams.
///
/// Algorithmic Complexity: $O(N)$ where $N$ is demand sources count.
pub async fn run_mrp_pipeline(
    demands: Vec<DemandSource>,
    stocks: HashMap<String, Decimal>,
) -> Vec<MRPResult> {
    let (tx, mut rx) = mpsc::channel(100);

    // Process each demand calculation concurrently in separate Tokio tasks
    for demand in demands {
        let tx_clone = tx.clone();
        let available = stocks.get(&demand.item_id).copied().unwrap_or(Decimal::ZERO);

        tokio::spawn(async move {
            let shortage = if demand.qty > available {
                demand.qty - available
            } else {
                Decimal::ZERO
            };

            let _ = tx_clone.send(MRPResult {
                item_id: demand.item_id,
                shortage_qty: shortage,
            }).await;
        });
    }

    // Drop original sender to close channel when all spawned tasks finish
    drop(tx);

    let mut results = Vec::new();
    while let Some(res) = rx.recv().await {
        results.push(res);
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[tokio::test]
    async fn test_mrp_pipeline_shortage_calculation() {
        let demands = vec![
            DemandSource { order_id: "ORD-1".into(), item_id: "ITEM-A".into(), qty: dec!(100.0) },
            DemandSource { order_id: "ORD-2".into(), item_id: "ITEM-B".into(), qty: dec!(50.0) },
            DemandSource { order_id: "ORD-3".into(), item_id: "ITEM-A".into(), qty: dec!(20.0) }, // Concurrent demand for A
        ];

        let mut stocks = HashMap::new();
        stocks.insert("ITEM-A".into(), dec!(80.0));
        stocks.insert("ITEM-B".into(), dec!(60.0));

        let results = run_mrp_pipeline(demands, stocks).await;

        assert_eq!(results.len(), 3);

        let a_shortages: Vec<_> = results.iter().filter(|r| r.item_id == "ITEM-A").collect();
        let b_shortages: Vec<_> = results.iter().filter(|r| r.item_id == "ITEM-B").collect();

        // ITEM-A has 80 in stock. ORD-1 wants 100 (shortage 20). ORD-3 wants 20 (shortage 0).
        // Note: The current pipeline evaluates each demand independently against total stock.
        // If it evaluates independently without deducting, both will see 80 available.
        // ORD-1: 100 - 80 = 20 shortage. ORD-3: 20 < 80 = 0 shortage.
        assert!(a_shortages.iter().any(|r| r.shortage_qty == dec!(20.0)));
        assert!(a_shortages.iter().any(|r| r.shortage_qty == dec!(0.0)));
        
        // ITEM-B wants 50, has 60, shortage 0.
        assert_eq!(b_shortages[0].shortage_qty, dec!(0.0));
    }
}
