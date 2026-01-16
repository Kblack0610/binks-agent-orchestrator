//! Memory information types

use serde::{Deserialize, Serialize};

/// Memory (RAM and swap) information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    /// Total physical memory in bytes
    pub total_bytes: u64,
    /// Used physical memory in bytes
    pub used_bytes: u64,
    /// Available physical memory in bytes
    pub available_bytes: u64,
    /// Memory usage percentage (0-100)
    pub usage_percent: f64,
    /// Total swap space in bytes
    pub swap_total_bytes: u64,
    /// Used swap space in bytes
    pub swap_used_bytes: u64,
    /// Swap usage percentage (0-100)
    pub swap_usage_percent: f64,
}
