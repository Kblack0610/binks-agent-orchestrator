//! Network information collection

use sysinfo::Networks;

use crate::types::{NetworkInfo, NetworkInterface};

/// Get network interface information
pub fn get_network_interfaces(interface_filter: Option<&str>) -> NetworkInfo {
    let networks = Networks::new_with_refreshed_list();

    let interfaces: Vec<NetworkInterface> = networks
        .iter()
        .filter(|(name, _)| {
            if let Some(filter) = interface_filter {
                name.contains(filter)
            } else {
                true
            }
        })
        .map(|(name, data)| NetworkInterface {
            name: name.clone(),
            mac_address: data.mac_address().to_string(),
            ip_addresses: data.ip_networks().iter().map(|ip| ip.addr.to_string()).collect(),
            total_received_bytes: data.total_received(),
            total_transmitted_bytes: data.total_transmitted(),
        })
        .collect();

    NetworkInfo { interfaces }
}
