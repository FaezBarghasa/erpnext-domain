use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Serialize, Deserialize};
use crate::StockError;
use crate::stock_ledger::{StockQueueNode, WarehouseStockState, ValuationMethod};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StockLedgerEntry {
    pub item_code: String,
    pub warehouse: String,
    pub qty_change: Decimal,
    pub rate: Decimal,
    pub timestamp: DateTime<Utc>,
}

pub struct BackdatedCorrectionEngine;

impl BackdatedCorrectionEngine {
    /// Recalculates stock quantities, FIFO queue state, and COGS downstream of a backdated entry.
    pub fn recalculate_valuation(
        initial_state: &WarehouseStockState,
        downstream_entries: &mut [StockLedgerEntry],
    ) -> Result<(WarehouseStockState, Vec<Decimal>), StockError> {
        let mut state = initial_state.clone();
        let mut recalculated_cogs = Vec::new();

        // Sort downstream entries chronologically
        downstream_entries.sort_by_key(|e| e.timestamp);

        for entry in downstream_entries {
            let new_qty = state.qty + entry.qty_change;
            let mut cogs = Decimal::ZERO;

            if entry.qty_change.is_sign_positive() {
                match state.valuation_method {
                    ValuationMethod::MovingAverage => {
                        if new_qty.is_zero() {
                            state.moving_average_rate = Decimal::ZERO;
                        } else {
                            state.moving_average_rate = ((state.qty * state.moving_average_rate)
                                + (entry.qty_change * entry.rate))
                                / new_qty;
                        }
                    }
                    ValuationMethod::FIFO => {
                        state.fifo_queue.push(StockQueueNode {
                            qty: entry.qty_change,
                            rate: entry.rate,
                            timestamp: entry.timestamp,
                        });
                        // Re-sort the queue on timestamp to maintain FIFO order after insertion
                        state.fifo_queue.sort_by_key(|n| n.timestamp);
                    }
                }
                state.qty = new_qty;
            } else if entry.qty_change.is_sign_negative() {
                let issue_qty = -entry.qty_change;
                match state.valuation_method {
                    ValuationMethod::MovingAverage => {
                        cogs = issue_qty * state.moving_average_rate;
                        state.qty = new_qty;
                    }
                    ValuationMethod::FIFO => {
                        cogs = crate::stock_ledger::consume_fifo_queue(&mut state.fifo_queue, issue_qty)?;
                        state.qty = new_qty;
                    }
                }
            }
            recalculated_cogs.push(cogs);
        }

        Ok((state, recalculated_cogs))
    }
}
