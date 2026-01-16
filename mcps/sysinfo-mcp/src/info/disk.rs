//! Disk information collection

use sysinfo::Disks;

use crate::types::{DiskInfo, Partition};

/// Get disk partition information
pub fn get_disk_info(mount_point_filter: Option<&str>) -> DiskInfo {
    let disks = Disks::new_with_refreshed_list();

    let partitions: Vec<Partition> = disks
        .iter()
        .filter(|disk| {
            if let Some(filter) = mount_point_filter {
                disk.mount_point().to_string_lossy().contains(filter)
            } else {
                true
            }
        })
        .map(|disk| {
            let total = disk.total_space();
            let available = disk.available_space();
            let used = total.saturating_sub(available);

            Partition {
                name: disk.name().to_string_lossy().to_string(),
                mount_point: disk.mount_point().to_string_lossy().to_string(),
                filesystem: disk.file_system().to_string_lossy().to_string(),
                total_bytes: total,
                available_bytes: available,
                used_bytes: used,
                usage_percent: if total > 0 {
                    (used as f64 / total as f64) * 100.0
                } else {
                    0.0
                },
                is_removable: disk.is_removable(),
            }
        })
        .collect();

    DiskInfo { disks: partitions }
}
