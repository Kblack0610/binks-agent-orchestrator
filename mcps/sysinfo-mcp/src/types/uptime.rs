//! Uptime information types

use serde::{Deserialize, Serialize};

/// System uptime information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UptimeInfo {
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Human-readable uptime (e.g., "2 days, 5 hours, 30 minutes")
    pub uptime_human: String,
    /// Unix timestamp of system boot time
    pub boot_time_unix: u64,
}
