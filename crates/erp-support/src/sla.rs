use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SLAConfig {
    pub response_time_limit: Duration,
    pub resolution_time_limit: Duration,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SupportTicket {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub assigned_at: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EscalationLevel {
    None,
    Level1, // Exceeded by up to 24h
    Level2, // Exceeded by up to 48h
    Level3, // Exceeded by > 48h
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SLAStatus {
    pub response_breached: bool,
    pub resolution_breached: bool,
    pub escalation: EscalationLevel,
}

pub fn calculate_sla_status(ticket: &SupportTicket, sla: &SLAConfig) -> SLAStatus {
    let now = Utc::now();
    let response_time = ticket.assigned_at.unwrap_or(now) - ticket.created_at;
    let response_time_std = response_time.to_std().unwrap_or(Duration::ZERO);
    let response_breached = response_time_std > sla.response_time_limit;

    let resolution_time = ticket.resolved_at.unwrap_or(now) - ticket.created_at;
    let resolution_time_std = resolution_time.to_std().unwrap_or(Duration::ZERO);
    let resolution_breached = resolution_time_std > sla.resolution_time_limit;

    let mut escalation = EscalationLevel::None;

    if resolution_breached {
        let exceeded_by = resolution_time_std - sla.resolution_time_limit;
        let exceeded_secs = Decimal::from(exceeded_by.as_secs());
        let hours_exceeded = exceeded_secs / dec!(3600);

        if hours_exceeded <= dec!(24) {
            escalation = EscalationLevel::Level1;
        } else if hours_exceeded <= dec!(48) {
            escalation = EscalationLevel::Level2;
        } else {
            escalation = EscalationLevel::Level3;
        }
    }

    SLAStatus {
        response_breached,
        resolution_breached,
        escalation,
    }
}
