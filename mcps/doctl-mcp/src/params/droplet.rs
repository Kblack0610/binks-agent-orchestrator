use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DropletListParams {
    #[schemars(description = "Filter droplets by tag name")]
    pub tag: Option<String>,

    #[schemars(description = "Filter droplets by region slug (e.g. nyc3, sfo3)")]
    pub region: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DropletGetParams {
    #[schemars(description = "Droplet ID")]
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DropletCreateParams {
    #[schemars(description = "Name for the new droplet")]
    pub name: String,

    #[schemars(description = "Region slug (e.g. nyc3, sfo3, ams3)")]
    pub region: String,

    #[schemars(description = "Size slug (e.g. s-1vcpu-1gb, s-2vcpu-4gb)")]
    pub size: String,

    #[schemars(description = "Image slug or ID (e.g. ubuntu-24-04-x64, debian-12-x64)")]
    pub image: String,

    #[schemars(description = "SSH key IDs or fingerprints (comma-separated)")]
    pub ssh_keys: Option<String>,

    #[schemars(description = "Tags to apply (comma-separated)")]
    pub tags: Option<String>,

    #[schemars(description = "Enable monitoring agent")]
    pub monitoring: Option<bool>,

    #[schemars(description = "VPC UUID to place the droplet in")]
    pub vpc_uuid: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DropletDeleteParams {
    #[schemars(description = "Droplet ID to delete")]
    pub id: String,

    #[schemars(description = "Skip confirmation (required for destructive action)")]
    pub force: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DropletActionParams {
    #[schemars(description = "Droplet ID")]
    pub id: String,

    #[schemars(
        description = "Action to perform: reboot, power_cycle, shutdown, power_off, power_on, rename, snapshot"
    )]
    pub action: String,

    #[schemars(description = "New name (required for rename action)")]
    pub name: Option<String>,

    #[schemars(description = "Snapshot name (required for snapshot action)")]
    pub snapshot_name: Option<String>,
}
