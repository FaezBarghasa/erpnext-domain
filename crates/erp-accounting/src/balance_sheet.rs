use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccountNode {
    pub account_id: String,
    pub parent_id: Option<String>,
    pub children: Vec<AccountNode>,
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
}
