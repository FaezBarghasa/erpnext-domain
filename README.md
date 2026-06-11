# erpnext-domain

Domain logic and ERP modules for the Rust ERPNext rewrite.

## Modules

The workspace is structured as a collection of domain-specific crates representing different modules:

- **`erp-accounting`**: Double-entry ledger posting and real-time balance sheet generation.
- **`erp-inventory`**: FIFO and Moving Average stock ledger valuation, backdated valuation correction engine, and SurrealDB-backed stock transfer transaction compiler with graph relation support.
- **`erp-manufacturing`**: Bill of Materials (BOM) cost rollups, recursive component expansions, workstation calculations, and Material Requirements Planning (MRP) demand/supply engines.
- **`erp-hr`**: Payroll processing with dynamic salary formula execution (via Rhai scripting), progressive income tax engines, and attendance tracking.
- **`erp-crm`**: Lead scoring engines and sales pipeline stage tracking.
- **`erp-support`**: Service Level Agreement (SLA) status tracking and resolution timers.
- **`erp-lending`**: Loan amortization scheduler supporting both EMI (Equal Monthly Installment) and Reducing Balance interest calculations.
- **`erp-trade`**: Customer pricing rules and taxation templates.
- **`erp-learning`**: Training and learning progress tracking.

## Getting Started

```bash
cargo build
cargo test --workspace
```
