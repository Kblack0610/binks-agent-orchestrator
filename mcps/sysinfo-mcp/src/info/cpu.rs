//! CPU information collection

use sysinfo::System;

use crate::types::{CoreUsage, CpuCore, CpuInfo, CpuUsage};

/// Get CPU hardware information
pub fn get_cpu_info(sys: &System, include_per_core: bool) -> CpuInfo {
    let cpus = sys.cpus();
    let first_cpu = cpus.first();

    let per_core = if include_per_core {
        Some(
            cpus.iter()
                .map(|cpu| CpuCore {
                    name: cpu.name().to_string(),
                    frequency_mhz: cpu.frequency(),
                })
                .collect(),
        )
    } else {
        None
    };

    CpuInfo {
        brand: first_cpu
            .map(|c| c.brand().to_string())
            .unwrap_or_default(),
        vendor_id: first_cpu
            .map(|c| c.vendor_id().to_string())
            .unwrap_or_default(),
        physical_cores: sys.physical_core_count(),
        logical_cores: cpus.len(),
        frequency_mhz: first_cpu.map(|c| c.frequency()).unwrap_or(0),
        per_core,
    }
}

/// Get current CPU usage
pub fn get_cpu_usage(sys: &System, per_core: bool) -> CpuUsage {
    let per_core_usage = if per_core {
        Some(
            sys.cpus()
                .iter()
                .map(|cpu| CoreUsage {
                    name: cpu.name().to_string(),
                    usage_percent: cpu.cpu_usage(),
                })
                .collect(),
        )
    } else {
        None
    };

    CpuUsage {
        global_usage_percent: sys.global_cpu_usage(),
        per_core_usage,
    }
}
