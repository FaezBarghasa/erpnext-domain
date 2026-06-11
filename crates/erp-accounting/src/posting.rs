use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use surrealdb::types::{RecordId, ToSql};
use crate::LedgerError;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GLEntry {
    pub account: RecordId,
    pub debit: Decimal,
    pub credit: Decimal,
    pub voucher_type: String,
    pub voucher_no: String,
    pub cost_center: Option<String>,
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
pub fn generate_posting_queries(entries: &[GLEntry]) -> Result<Vec<String>, LedgerError> {
    validate_and_compile_transaction(entries)?;

    let mut queries = vec!["BEGIN TRANSACTION;".to_string()];

    for entry in entries {
        let account_str = entry.account.to_sql();
        let cost_center_val = match &entry.cost_center {
            Some(cc) => format!("'{}'", cc),
            None => "NONE".to_string(),
        };

        // Insert General Ledger Entry
        queries.push(format!(
            "CREATE gl_entry CONTENT {{ account: {}, debit: {}, credit: {}, voucher_type: '{}', voucher_no: '{}', cost_center: {} }};",
            account_str, entry.debit, entry.credit, entry.voucher_type, entry.voucher_no, cost_center_val
        ));
        
        // Update account balances: balance = balance + debit - credit
        queries.push(format!(
            "UPDATE {} SET balance = balance + {} - {};",
            account_str, entry.debit, entry.credit
        ));
    }

    queries.push("COMMIT TRANSACTION;".to_string());
    Ok(queries)
}

pub struct LedgerPostingEngine;

impl LedgerPostingEngine {
    /// Commits double-entry ledger entries along with a parent voucher record using SurrealQL graph edges.
    pub async fn commit_transaction<C: surrealdb::Connection>(
        db: &surrealdb::Surreal<C>,
        ns: &str,
        database: &str,
        voucher_type: &str,
        voucher_no: &str,
        entries: &[GLEntry],
    ) -> Result<(), LedgerError> {
        validate_and_compile_transaction(entries)?;

        db.use_ns(ns).use_db(database).await.map_err(|e| LedgerError::Database(e.to_string()))?;

        let voucher_id = RecordId::parse_simple(&format!("{}:{}", voucher_type, voucher_no))
            .map_err(|e| LedgerError::Database(e.to_string()))?;
        let voucher_sql = voucher_id.to_sql();

        let mut queries = vec![
            "BEGIN TRANSACTION;".to_string(),
            format!("let $voucher = {};", voucher_sql),
        ];

        for (i, entry) in entries.iter().enumerate() {
            let account_sql = entry.account.to_sql();
            let cost_center_val = match &entry.cost_center {
                Some(cc) => format!("'{}'", cc),
                None => "NONE".to_string(),
            };

            queries.push(format!(
                "let $entry{} = (CREATE gl_entry CONTENT {{ account: {}, debit: {}, credit: {}, voucher_type: '{}', voucher_no: '{}', cost_center: {} }});",
                i, account_sql, entry.debit, entry.credit, entry.voucher_type, entry.voucher_no, cost_center_val
            ));

            queries.push(format!(
                "RELATE $voucher -> POSTED_TO -> {} CONTENT {{ debit: {}, credit: {}, entry: $entry{}.id }};",
                account_sql, entry.debit, entry.credit, i
            ));

            queries.push(format!(
                "UPDATE {} SET balance = balance + {} - {};",
                account_sql, entry.debit, entry.credit
            ));
        }

        queries.push("COMMIT TRANSACTION;".to_string());

        let tx_query = queries.join("\n");
        let res = db.query(&tx_query).await.map_err(|e| LedgerError::Database(e.to_string()))?;
        res.check().map_err(|e| LedgerError::Database(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_transaction() {
        let entries = vec![
            GLEntry {
                account: RecordId::parse_simple("account:asset").unwrap(),
                debit: Decimal::new(100, 0),
                credit: Decimal::ZERO,
                voucher_type: "Sales Invoice".to_string(),
                voucher_no: "SINV-001".to_string(),
                cost_center: None,
            },
            GLEntry {
                account: RecordId::parse_simple("account:revenue").unwrap(),
                debit: Decimal::ZERO,
                credit: Decimal::new(100, 0),
                voucher_type: "Sales Invoice".to_string(),
                voucher_no: "SINV-001".to_string(),
                cost_center: None,
            },
        ];
        assert!(validate_and_compile_transaction(&entries).is_ok());
    }

    #[test]
    fn test_invalid_transaction() {
        let entries = vec![
            GLEntry {
                account: RecordId::parse_simple("account:asset").unwrap(),
                debit: Decimal::new(100, 0),
                credit: Decimal::ZERO,
                voucher_type: "Sales Invoice".to_string(),
                voucher_no: "SINV-001".to_string(),
                cost_center: None,
            },
            GLEntry {
                account: RecordId::parse_simple("account:revenue").unwrap(),
                debit: Decimal::ZERO,
                credit: Decimal::new(99, 0),
                voucher_type: "Sales Invoice".to_string(),
                voucher_no: "SINV-001".to_string(),
                cost_center: None,
            },
        ];
        assert!(validate_and_compile_transaction(&entries).is_err());
    }

    #[test]
    fn test_generate_posting_queries() {
        let entries = vec![
            GLEntry {
                account: RecordId::parse_simple("account:cash").unwrap(),
                debit: Decimal::new(200, 0),
                credit: Decimal::ZERO,
                voucher_type: "Sales Invoice".to_string(),
                voucher_no: "SINV-002".to_string(),
                cost_center: None,
            },
            GLEntry {
                account: RecordId::parse_simple("account:equity").unwrap(),
                debit: Decimal::ZERO,
                credit: Decimal::new(200, 0),
                voucher_type: "Sales Invoice".to_string(),
                voucher_no: "SINV-002".to_string(),
                cost_center: None,
            },
        ];
        
        let queries = generate_posting_queries(&entries).unwrap();
        assert_eq!(queries.len(), 6); // BEGIN, CREATE, UPDATE, CREATE, UPDATE, COMMIT
        assert_eq!(queries[0], "BEGIN TRANSACTION;");
        assert_eq!(queries[5], "COMMIT TRANSACTION;");

        // Now test unbalanced
        let bad_entries = vec![
            GLEntry {
                account: RecordId::parse_simple("account:cash").unwrap(),
                debit: Decimal::new(200, 0),
                credit: Decimal::ZERO,
                voucher_type: "Sales Invoice".to_string(),
                voucher_no: "SINV-002".to_string(),
                cost_center: None,
            },
        ];
        assert!(generate_posting_queries(&bad_entries).is_err());
    }
}
