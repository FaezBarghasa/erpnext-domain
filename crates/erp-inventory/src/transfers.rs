use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StockTransferCmd {
    pub source_warehouse: String,
    pub target_warehouse: String,
    pub item_id: String,
    pub qty: Decimal,
    pub rate: Decimal,
}

/// Generates transactional SurrealQL queries including graph relations for stock transfers.
///
/// Algorithmic Complexity: $O(1)$ query formulation.
pub fn compile_stock_transfer_queries(cmd: &StockTransferCmd) -> Vec<String> {
    let mut queries = vec!["BEGIN TRANSACTION;".to_string()];

    let source = &cmd.source_warehouse;
    let target = &cmd.target_warehouse;
    let item = &cmd.item_id;
    let qty = cmd.qty;
    let rate = cmd.rate;

    // 1. Create a transfer event
    let transfer_id = format!("stock_transfer:{}", uuid::Uuid::new_v4());
    queries.push(format!(
        "CREATE {} SET item = {}, qty = {}, rate = {};",
        transfer_id, item, qty, rate
    ));

    // 2. Outgoing inventory adjustment (reduce from source)
    queries.push(format!(
        "UPDATE warehouse:{} SET inventory = inventory - {} WHERE item = {};",
        source, qty, item
    ));

    // 3. Incoming inventory adjustment (add to target)
    queries.push(format!(
        "UPDATE warehouse:{} SET inventory = inventory + {} WHERE item = {};",
        target, qty, item
    ));

    // 4. Relate transactions via graph edges
    queries.push(format!(
        "RELATE {} -> shipped_from -> warehouse:{} CONTENT {{ qty: {} }};",
        transfer_id, source, qty
    ));
    queries.push(format!(
        "RELATE {} -> received_at -> warehouse:{} CONTENT {{ qty: {} }};",
        transfer_id, target, qty
    ));

    queries.push("COMMIT TRANSACTION;".to_string());
    queries
}
