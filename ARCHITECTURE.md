# Architecture: erpnext-domain

This workspace implements the business logic domain layer for the Rust ERPNext rewrite, utilizing SurrealDB for multi-model queries and graph relations.

## Architectural Features

1. **High-Precision Calculations**:
   - To prevent float drift errors in accounting and inventory calculations, all currency balances, tax rates, routing costs, and quantities are modeled using high-precision decimal representation via `rust_decimal::Decimal`.

2. **FIFO/Moving Average & Backdated Valuation Correction**:
   - The stock ledger supports FIFO and Moving Average valuation methods.
   - A `BackdatedCorrectionEngine` recalculates quantities, FIFO queue states, and COGS downstream of backdated stock entries chronologically.
   - Transactional SurrealQL queries represent stock transfers, updating database tables and generating graph edges (`MOVED_FROM`, `MOVED_TO`) within a single database transaction.

3. **Loan Amortization Scheduler**:
   - The lending module computes amortization schedules dynamically.
   - Supports **EMI (Equal Monthly Installments)** with fixed periodic payments.
   - Supports **Reducing Balance** interest with fixed principal payments and decreasing total payments, avoiding rounding drift.

4. **Dynamic Formulas via Rhai**:
   - Payroll salary components are evaluated dynamically using a sandboxed Rhai scripting engine.
