//! OS information collection

use sysinfo::System;

use crate::types::OsInfo;

/// Get operating system information
pub fn get_os_info() -> OsInfo {
    OsInfo {
        name: System::name(),
        version: System::os_version(),
        kernel_version: System::kernel_version(),
        hostname: System::host_name(),
        architecture: std::env::consts::ARCH.to_string(),
    }
}
