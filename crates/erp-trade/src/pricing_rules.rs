use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PricingRule {
    pub discount_percentage: Decimal,
    pub min_qty: Decimal,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LineItem {
    pub id: String,
    pub qty: Decimal,
    pub base_rate: Decimal,
}
