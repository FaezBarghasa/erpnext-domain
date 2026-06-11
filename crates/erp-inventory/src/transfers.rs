use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use surrealdb::Surreal;

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

/// Executes a stock transfer by updating memory registry and writing database ledger records using graph relations.
pub async fn execute_transfer<C: surrealdb::Connection>(
    db: &Surreal<C>,
    ns: &str,
    database: &str,
    registry: &crate::stock_ledger::WarehouseStockRegistry,
    from_warehouse: &str,
    to_warehouse: &str,
    item: &str,
    qty: Decimal,
    rate: Decimal,
) -> Result<(), crate::StockError> {
    // 1. Call update_stock for both source (negative) and target (positive) warehouses in memory
    let now = chrono::Utc::now();
    registry.update_stock(item, from_warehouse, -qty, rate, now)?;
    registry.update_stock(item, to_warehouse, qty, rate, now)?;

    db.use_ns(ns).use_db(database).await.map_err(|e| crate::StockError::Database(e.to_string()))?;

    // 2. Generate SurrealDB database transaction with MOVED_FROM and MOVED_TO graph edges
    let transfer_id = format!("stock_transfer:{}", uuid::Uuid::new_v4());
    let query = format!(
        "BEGIN TRANSACTION;\n\
         CREATE {} SET item = '{}', qty = {}, rate = {};\n\
         UPDATE warehouse:{} SET inventory = inventory - {} WHERE item = '{}';\n\
         UPDATE warehouse:{} SET inventory = inventory + {} WHERE item = '{}';\n\
         RELATE {} -> MOVED_FROM -> warehouse:{} CONTENT {{ qty: {} }};\n\
         RELATE {} -> MOVED_TO -> warehouse:{} CONTENT {{ qty: {} }};\n\
         COMMIT TRANSACTION;",
        transfer_id, item, qty, rate,
        from_warehouse, qty, item,
        to_warehouse, qty, item,
        transfer_id, from_warehouse, qty,
        transfer_id, to_warehouse, qty
    );

    let res = db.query(&query).await.map_err(|e| crate::StockError::Database(e.to_string()))?;
    res.check().map_err(|e| crate::StockError::Database(e.to_string()))?;

    Ok(())
}
