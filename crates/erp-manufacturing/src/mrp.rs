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
