use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use crate::pricing_rules::{LineItem, PricingRule};
use rust_decimal_macros::dec;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaxTemplate {
    pub rate_percentage: Decimal,
    pub compound: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DocumentTotals {
    pub net_total: Decimal,
    pub total_taxes: Decimal,
    pub grand_total: Decimal,
}

pub fn calculate_document_totals(
    items: &[LineItem],
    taxes: &[TaxTemplate],
    pricing: &[PricingRule],
) -> DocumentTotals {
    let mut net_total = Decimal::ZERO;

    for item in items {
        let mut item_rate = item.base_rate;
        for rule in pricing {
            if item.qty >= rule.min_qty {
                let discount = item_rate * (rule.discount_percentage / dec!(100));
                item_rate -= discount;
            }
        }
        net_total += item.qty * item_rate;
    }

    let mut total_taxes = Decimal::ZERO;
    let mut current_taxable_amount = net_total;

    for tax in taxes {
        let tax_amount = current_taxable_amount * (tax.rate_percentage / dec!(100));
        total_taxes += tax_amount;
        if tax.compound {
            current_taxable_amount += tax_amount;
        }
    }

    DocumentTotals {
        net_total,
        total_taxes,
        grand_total: net_total + total_taxes,
    }
}
