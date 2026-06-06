pub mod stock_ledger;
pub mod transfers;

#[derive(thiserror::Error, Debug)]
pub enum StockError {
    #[error("Insufficient stock quantity")]
    InsufficientStock,
    #[error("Database error: {0}")]
    Database(String),
}
