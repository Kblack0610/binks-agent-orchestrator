//! System information collection modules

pub mod cpu;
pub mod disk;
pub mod memory;
pub mod network;
pub mod os;
pub mod uptime;

use sysinfo::System;

use crate::types::SystemSummary;

/// Get a complete system summary
pub fn get_system_summary(sys: &System) -> SystemSummary {
    SystemSummary {
        os: os::get_os_info(),
        cpu: cpu::get_cpu_info(sys, false),
        cpu_usage: cpu::get_cpu_usage(sys, false),
        memory: memory::get_memory_info(sys),
        disks: disk::get_disk_info(None),
        network: network::get_network_interfaces(None),
        uptime: uptime::get_uptime(),
    }
}
