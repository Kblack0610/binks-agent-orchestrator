//! CPU information types

use serde::{Deserialize, Serialize};

/// CPU hardware information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
    /// CPU brand/model name
    pub brand: String,
    /// CPU vendor ID
    pub vendor_id: String,
    /// Number of physical CPU cores
    pub physical_cores: Option<usize>,
    /// Number of logical CPU cores (including hyperthreading)
    pub logical_cores: usize,
    /// CPU frequency in MHz
    pub frequency_mhz: u64,
    /// Per-core information (if requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_core: Option<Vec<CpuCore>>,
}

/// Individual CPU core information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuCore {
    /// Core name/identifier
    pub name: String,
    /// Core frequency in MHz
    pub frequency_mhz: u64,
}

/// CPU usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuUsage {
    /// Global CPU usage percentage (0-100)
    pub global_usage_percent: f32,
    /// Per-core usage (if requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_core_usage: Option<Vec<CoreUsage>>,
}

/// Individual CPU core usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreUsage {
    /// Core name/identifier
    pub name: String,
    /// Core usage percentage (0-100)
    pub usage_percent: f32,
}
