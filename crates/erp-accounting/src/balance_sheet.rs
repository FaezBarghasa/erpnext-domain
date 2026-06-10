use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccountNode {
    pub account_id: String,
    pub parent_id: Option<String>,
    pub children: Vec<AccountNode>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccountRecord {
    pub id: String,
    pub parent: Option<String>,
    pub name: String,
    pub balance: Decimal,
}

/// Recursively calculates and rolls up balances from leaf accounts to parent composite accounts.
///
/// LaTeX:
/// $$\text{AccountBalance} = \text{SelfBalance} + \sum \text{ChildBalances}$$
///
/// Algorithmic Complexity: $O(V + E)$ tree traversal where $V$ is number of accounts.
pub fn calculate_composite_balance(
    node: &AccountNode,
    ledger_data: &BTreeMap<String, Decimal>,
    balance_tree: &mut BTreeMap<String, Decimal>,
) -> Decimal {
    let self_balance = ledger_data.get(&node.account_id).copied().unwrap_or(Decimal::ZERO);
    let mut total_balance = self_balance;

    for child in &node.children {
        total_balance += calculate_composite_balance(child, ledger_data, balance_tree);
    }

    balance_tree.insert(node.account_id.clone(), total_balance);
    total_balance
}

/// Build SurrealQL query to fetch the entire CoA tree structure recursively.
pub fn generate_coa_recursive_query() -> &'static str {
    "SELECT id, parent, name, balance FROM account FETCH parent;"
}

/// Construct a tree of AccountNodes from a flat list of AccountRecords.
///
/// Algorithmic Complexity: $O(N)$ where $N$ is number of records.
pub fn build_account_tree(records: &[AccountRecord]) -> Vec<AccountNode> {
    // 1. Group records by parent key
    let mut by_parent: HashMap<Option<String>, Vec<AccountRecord>> = HashMap::new();
    for rec in records {
        by_parent.entry(rec.parent.clone()).or_default().push(rec.clone());
    }

    // Recursive helper function to build subtrees
    fn build_node(
        record: &AccountRecord,
        by_parent: &HashMap<Option<String>, Vec<AccountRecord>>,
    ) -> AccountNode {
        let parent_key = Some(record.id.clone());
        let mut children = Vec::new();
        if let Some(children_recs) = by_parent.get(&parent_key) {
            for child_rec in children_recs {
                children.push(build_node(child_rec, by_parent));
            }
        }
        AccountNode {
            account_id: record.id.clone(),
            parent_id: record.parent.clone(),
            children,
        }
    }

    // 2. Identify roots: parents not present in the record IDs list, or is None
    let all_ids: HashSet<String> = records.iter().map(|r| r.id.clone()).collect();
    let mut roots = Vec::new();
    for rec in records {
        let is_root = match &rec.parent {
            None => true,
            Some(p) => !all_ids.contains(p),
        };
        if is_root {
            roots.push(build_node(rec, &by_parent));
        }
    }

    roots
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_composite_rollup() {
        // Build CoA tree:
        // Asset (composite)
        //   ├── Cash (leaf)
        //   └── Bank (leaf)
        let cash_node = AccountNode {
            account_id: "cash".to_string(),
            parent_id: Some("asset".to_string()),
            children: vec![],
        };
        let bank_node = AccountNode {
            account_id: "bank".to_string(),
            parent_id: Some("asset".to_string()),
            children: vec![],
        };
        let asset_node = AccountNode {
            account_id: "asset".to_string(),
            parent_id: None,
            children: vec![cash_node, bank_node],
        };

        let mut ledger = BTreeMap::new();
        ledger.insert("cash".to_string(), Decimal::new(150, 0));
        ledger.insert("bank".to_string(), Decimal::new(350, 0));

        let mut balance_tree = BTreeMap::new();
        let total = calculate_composite_balance(&asset_node, &ledger, &mut balance_tree);

        assert_eq!(total, Decimal::new(500, 0));
        assert_eq!(balance_tree.get("cash").copied(), Some(Decimal::new(150, 0)));
        assert_eq!(balance_tree.get("bank").copied(), Some(Decimal::new(350, 0)));
        assert_eq!(balance_tree.get("asset").copied(), Some(Decimal::new(500, 0)));
    }

    #[test]
    fn test_build_account_tree_and_rollup() {
        let records = vec![
            AccountRecord {
                id: "asset".to_string(),
                parent: None,
                name: "Asset".to_string(),
                balance: Decimal::ZERO,
            },
            AccountRecord {
                id: "cash".to_string(),
                parent: Some("asset".to_string()),
                name: "Cash".to_string(),
                balance: Decimal::new(150, 0),
            },
            AccountRecord {
                id: "bank".to_string(),
                parent: Some("asset".to_string()),
                name: "Bank".to_string(),
                balance: Decimal::new(350, 0),
            },
        ];

        let roots = build_account_tree(&records);
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].account_id, "asset");
        assert_eq!(roots[0].children.len(), 2);

        let mut ledger_data = BTreeMap::new();
        for rec in &records {
            ledger_data.insert(rec.id.clone(), rec.balance);
        }

        let mut balance_tree = BTreeMap::new();
        let total = calculate_composite_balance(&roots[0], &ledger_data, &mut balance_tree);

        assert_eq!(total, Decimal::new(500, 0));
        assert_eq!(balance_tree.get("asset").copied(), Some(Decimal::new(500, 0)));
    }
}
