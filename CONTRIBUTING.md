# Contributing to erpnext-domain

1. **Precision First**: Always use `rust_decimal::Decimal` for financial, tax, or inventory quantities. Never use `f32` or `f64`.
2. **Thread Safety**: Ensure all shared state is thread-safe. Avoid global locks.
3. **No Placeholders**: Do not use `todo!` macros or empty placeholders in production-ready files.
4. **Verification**: Always run `cargo fmt`, `cargo clippy`, and `cargo test` prior to submitting changes.
