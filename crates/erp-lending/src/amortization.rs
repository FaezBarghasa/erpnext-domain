use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use rust_decimal_macros::dec;
use rust_decimal::MathematicalOps;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AmortizationRow {
    pub period: u32,
    pub opening_balance: Decimal,
    pub principal_payment: Decimal,
    pub interest_payment: Decimal,
    pub closing_balance: Decimal,
}

pub fn calculate_amortization(
    principal: Decimal,
    annual_rate: Decimal,
    periods: u32,
) -> Vec<AmortizationRow> {
    if periods == 0 {
        return vec![];
    }

    let r = annual_rate / dec!(12);
    let mut schedule = Vec::new();

    // M = P * [ r * (1+r)^n ] / [ (1+r)^n - 1 ]
    let one_plus_r = dec!(1) + r;
    // powu instead of powi because it takes an unsigned int, but let's check what rust_decimal provides.
    // Let's use powu.
    let one_plus_r_pow_n = one_plus_r.powu(periods as u64);

    let m = if r.is_zero() {
        principal / Decimal::from(periods)
    } else {
        principal * (r * one_plus_r_pow_n) / (one_plus_r_pow_n - dec!(1))
    };

    let mut current_balance = principal;

    for i in 1..=periods {
        let interest = current_balance * r;
        let mut principal_payment = m - interest;

        // Adjust for last period to avoid rounding errors
        if i == periods {
            principal_payment = current_balance;
        }

        let closing_balance = current_balance - principal_payment;

        schedule.push(AmortizationRow {
            period: i,
            opening_balance: current_balance,
            principal_payment,
            interest_payment: interest,
            closing_balance,
        });

        current_balance = closing_balance;
    }

    schedule
}
