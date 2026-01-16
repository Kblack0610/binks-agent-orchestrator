//! Disk information types

use serde::{Deserialize, Serialize};

/// Disk information containing all partitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    /// List of disk partitions
    pub disks: Vec<Partition>,
}

/// Individual disk partition information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Partition {
    /// Disk/partition name
    pub name: String,
    /// Mount point path
    pub mount_point: String,
    /// Filesystem type (e.g., "ext4", "ntfs", "apfs")
    pub filesystem: String,
    /// Total space in bytes
    pub total_bytes: u64,
    /// Available space in bytes
    pub available_bytes: u64,
    /// Used space in bytes
    pub used_bytes: u64,
    /// Usage percentage (0-100)
    pub usage_percent: f64,
    /// Whether the disk is removable
    pub is_removable: bool,
}
