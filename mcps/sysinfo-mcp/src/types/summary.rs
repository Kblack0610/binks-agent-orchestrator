//! System summary type combining all info

use serde::{Deserialize, Serialize};

use super::{CpuInfo, CpuUsage, DiskInfo, MemoryInfo, NetworkInfo, OsInfo, UptimeInfo};

/// Combined system information summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSummary {
    /// Operating system information
    pub os: OsInfo,
    /// CPU hardware information
    pub cpu: CpuInfo,
    /// Current CPU usage
    pub cpu_usage: CpuUsage,
    /// Memory information
    pub memory: MemoryInfo,
    /// Disk information
    pub disks: DiskInfo,
    /// Network interfaces
    pub network: NetworkInfo,
    /// System uptime
    pub uptime: UptimeInfo,
}
