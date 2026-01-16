//! Uptime information collection

use sysinfo::System;

use crate::types::UptimeInfo;

/// Get system uptime information
pub fn get_uptime() -> UptimeInfo {
    let uptime_secs = System::uptime();
    let boot_time = System::boot_time();

    UptimeInfo {
        uptime_seconds: uptime_secs,
        uptime_human: format_uptime(uptime_secs),
        boot_time_unix: boot_time,
    }
}

/// Format uptime seconds into human-readable string
fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    let mut parts = Vec::new();
    if days > 0 {
        parts.push(format!("{} day{}", days, if days == 1 { "" } else { "s" }));
    }
    if hours > 0 {
        parts.push(format!("{} hour{}", hours, if hours == 1 { "" } else { "s" }));
    }
    if minutes > 0 {
        parts.push(format!(
            "{} minute{}",
            minutes,
            if minutes == 1 { "" } else { "s" }
        ));
    }
    if secs > 0 || parts.is_empty() {
        parts.push(format!("{} second{}", secs, if secs == 1 { "" } else { "s" }));
    }

    parts.join(", ")
}
