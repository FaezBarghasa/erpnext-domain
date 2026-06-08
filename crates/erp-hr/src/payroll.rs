use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SalaryStructure {
    pub base_pay: Decimal,
    pub earnings: Vec<Decimal>,
    pub deductions: Vec<Decimal>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SalarySlipResult {
    pub gross_pay: Decimal,
    pub total_deductions: Decimal,
    pub net_pay: Decimal,
}

pub fn compute_salary_slip(
    attendance_rate: Decimal,
    structure: &SalaryStructure,
) -> SalarySlipResult {
    let mut gross_pay = structure.base_pay * attendance_rate;
    for earning in &structure.earnings {
        gross_pay += earning * attendance_rate;
    }

    let mut total_deductions = Decimal::ZERO;
    for deduction in &structure.deductions {
        total_deductions += deduction;
    }

    let net_pay = gross_pay - total_deductions;

    SalarySlipResult {
        gross_pay,
        total_deductions,
        net_pay,
    }
}
