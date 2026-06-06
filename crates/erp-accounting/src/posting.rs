use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use surrealdb::Thing as RecordId;
use crate::LedgerError;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GLEntry {
    pub account: RecordId,
    pub debit: Decimal,
    pub credit: Decimal,
}

/// Enforces the double-entry accounting rule and returns unbalanced discrepancy if any.
///
/// LaTeX:
/// $$\sum \text{Debits} - \sum \text{Credits} = 0$$
///
/// Algorithmic Complexity: $O(N)$ validation loop.
pub fn validate_and_compile_transaction(entries: &[GLEntry]) -> Result<(), LedgerError> {
    let mut sum_debits = Decimal::ZERO;
    let mut sum_credits = Decimal::ZERO;

    for entry in entries {
        sum_debits += entry.debit;
        sum_credits += entry.credit;
    }

    let diff = sum_debits - sum_credits;
    if !diff.is_zero() {
        return Err(LedgerError::UnbalancedEntry(diff));
    }

    Ok(())
}

/// Generates SurrealQL queries to record the transaction and update target account balances.
pub fn generate_posting_queries(entries: &[GLEntry]) -> Vec<String> {
    let mut queries = vec!["BEGIN TRANSACTION;".to_string()];

    for entry in entries {
        let account_str = format!("{}", entry.account);
        // Insert General Ledger Entry
        queries.push(format!(
            "CREATE gl_entry CONTENT {{ account: {}, debit: {}, credit: {} }};",
            account_str, entry.debit, entry.credit
        ));
        
        // Update account balances: balance = balance + debit - credit
        queries.push(format!(
            "UPDATE {} SET balance = balance + {} - {};",
            account_str, entry.debit, entry.credit
        ));
    }

    queries.push("COMMIT TRANSACTION;".to_string());
    queries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_transaction() {
        let entries = vec![
            GLEntry {
                account: RecordId::from_string("account:asset").unwrap(),
                debit: Decimal::new(100, 0),
                credit: Decimal::ZERO,
            },
            GLEntry {
                account: RecordId::from_string("account:revenue").unwrap(),
                debit: Decimal::ZERO,
                credit: Decimal::new(100, 0),
            },
        ];
        assert!(validate_and_compile_transaction(&entries).is_ok());
    }

    #[test]
    fn test_invalid_transaction() {
        let entries = vec![
            GLEntry {
                account: RecordId::from_string("account:asset").unwrap(),
                debit: Decimal::new(100, 0),
                credit: Decimal::ZERO,
            },
            GLEntry {
                account: RecordId::from_string("account:revenue").unwrap(),
                debit: Decimal::ZERO,
                credit: Decimal::new(99, 0),
            },
        ];
        assert!(validate_and_compile_transaction(&entries).is_err());
    }
}
