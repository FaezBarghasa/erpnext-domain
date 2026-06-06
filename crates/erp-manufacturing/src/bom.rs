use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BOMComponent {
    pub child_node: BOMNode,
    pub qty: Decimal,
    pub rate: Decimal,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BOMNode {
    pub item_id: String,
    pub is_phantom: bool,
    pub components: Vec<BOMComponent>,
    pub routing_costs: Vec<Decimal>,
}

/// Recursively calculates the total cost of a Bill of Materials (BOM), expanding phantom nodes inline.
///
/// LaTeX:
/// $$\text{TotalParentCost} = \sum \left( \text{Qty}_{\text{Component}} \times \text{Rate}_{\text{Component}} \right) + \sum \text{RoutingCost}$$
///
/// Algorithmic Complexity: $O(V + E)$ where $V$ is number of components in DAG. Zero allocation on lookup path.
pub fn calculate_bom_cost(node: &BOMNode) -> Decimal {
    let mut component_costs = Decimal::ZERO;

    for comp in &node.components {
        if comp.child_node.is_phantom {
            // Expand phantom component inline by computing its unit cost recursively
            let phantom_unit_cost = calculate_bom_cost(&comp.child_node);
            component_costs += comp.qty * phantom_unit_cost;
        } else {
            component_costs += comp.qty * comp.rate;
        }
    }

    let routing_total: Decimal = node.routing_costs.iter().sum();

    component_costs + routing_total
}

/// Generates SurrealQL query to fetch direct component nodes recursively.
pub fn generate_bom_dag_query(parent_item: &str) -> String {
    format!(
        "SELECT ->requires->bom_node.* AS components FROM item:{} FETCH components;",
        parent_item
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phantom_bom_cost() {
        // Child phantom: 2 * $5 component + $2 routing = $12 unit cost
        let child_comp = BOMComponent {
            child_node: BOMNode {
                item_id: "raw_material".to_string(),
                is_phantom: false,
                components: vec![],
                routing_costs: vec![],
            },
            qty: Decimal::new(2, 0),
            rate: Decimal::new(5, 0),
        };
        let phantom_node = BOMNode {
            item_id: "phantom_assembly".to_string(),
            is_phantom: true,
            components: vec![child_comp],
            routing_costs: vec![Decimal::new(2, 0)],
        };

        // Parent node: 3 * phantom_assembly + $10 routing
        // Total cost = 3 * 12 + 10 = $46
        let parent_comp = BOMComponent {
            child_node: phantom_node,
            qty: Decimal::new(3, 0),
            rate: Decimal::ZERO, // Rate is ignored for phantom nodes
        };

        let parent_node = BOMNode {
            item_id: "finished_product".to_string(),
            is_phantom: false,
            components: vec![parent_comp],
            routing_costs: vec![Decimal::new(10, 0)],
        };

        let total_cost = calculate_bom_cost(&parent_node);
        assert_eq!(total_cost, Decimal::new(46, 0));
    }
}
