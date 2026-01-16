//! OS information types

use serde::{Deserialize, Serialize};

/// Operating system information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsInfo {
    /// OS name (e.g., "Linux", "Windows", "macOS")
    pub name: Option<String>,
    /// OS version
    pub version: Option<String>,
    /// Kernel version
    pub kernel_version: Option<String>,
    /// Hostname
    pub hostname: Option<String>,
    /// CPU architecture (e.g., "x86_64", "aarch64")
    pub architecture: String,
}
