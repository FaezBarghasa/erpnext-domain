use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Lead {
    pub id: String,
    pub profile_completeness: Decimal,
    pub engagement_score: Decimal,
    pub fit_score: Decimal,
}

pub fn calculate_lead_score(
    lead: &Lead,
    profile_weight: Decimal,
    engagement_weight: Decimal,
    fit_weight: Decimal,
) -> Decimal {
    let score = (lead.profile_completeness * profile_weight)
        + (lead.engagement_score * engagement_weight)
        + (lead.fit_score * fit_weight);

    let total_weight = profile_weight + engagement_weight + fit_weight;
    if total_weight.is_zero() {
        return Decimal::ZERO;
    }

    score / total_weight
}
