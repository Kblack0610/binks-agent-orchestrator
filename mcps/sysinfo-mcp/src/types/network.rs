//! Network information types

use serde::{Deserialize, Serialize};

/// Network information containing all interfaces
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    /// List of network interfaces
    pub interfaces: Vec<NetworkInterface>,
}

/// Individual network interface information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    /// Interface name (e.g., "eth0", "wlan0", "en0")
    pub name: String,
    /// MAC address
    pub mac_address: String,
    /// IP addresses assigned to this interface
    pub ip_addresses: Vec<String>,
    /// Total bytes received since boot
    pub total_received_bytes: u64,
    /// Total bytes transmitted since boot
    pub total_transmitted_bytes: u64,
}
