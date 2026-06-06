pub mod posting;
pub mod balance_sheet;

#[derive(thiserror::Error, Debug)]
pub enum LedgerError {
    #[error("Unbalanced entry with discrepancy: {0}")]
    UnbalancedEntry(rust_decimal::Decimal),
    #[error("Database error: {0}")]
    Database(String),
}
