use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ErrorData as McpError,
};

use crate::handlers;
use crate::params::*;

#[derive(Clone)]
pub struct DoctlMcpServer {
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl DoctlMcpServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    // === Droplet Tools ===

    #[tool(description = "List all droplets. Optionally filter by tag or region.")]
    async fn doctl_droplet_list(
        &self,
        Parameters(params): Parameters<DropletListParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::droplet::list(params).await
    }

    #[tool(description = "Get details of a specific droplet by ID")]
    async fn doctl_droplet_get(
        &self,
        Parameters(params): Parameters<DropletGetParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::droplet::get(params).await
    }

    #[tool(description = "Create a new droplet")]
    async fn doctl_droplet_create(
        &self,
        Parameters(params): Parameters<DropletCreateParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::droplet::create(params).await
    }

    #[tool(description = "Delete a droplet by ID. Use force=true to skip confirmation.")]
    async fn doctl_droplet_delete(
        &self,
        Parameters(params): Parameters<DropletDeleteParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::droplet::delete(params).await
    }

    #[tool(
        description = "Perform an action on a droplet (reboot, power_cycle, shutdown, power_off, power_on, rename, snapshot)"
    )]
    async fn doctl_droplet_action(
        &self,
        Parameters(params): Parameters<DropletActionParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::droplet::action(params).await
    }

    // === Kubernetes Tools ===

    #[tool(description = "List all Kubernetes clusters")]
    async fn doctl_k8s_cluster_list(
        &self,
        Parameters(params): Parameters<K8sClusterListParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::kubernetes::cluster_list(params).await
    }

    #[tool(description = "Get details of a specific Kubernetes cluster")]
    async fn doctl_k8s_cluster_get(
        &self,
        Parameters(params): Parameters<K8sClusterGetParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::kubernetes::cluster_get(params).await
    }

    #[tool(description = "Get or save kubeconfig for a Kubernetes cluster")]
    async fn doctl_k8s_kubeconfig(
        &self,
        Parameters(params): Parameters<K8sKubeconfigParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::kubernetes::kubeconfig(params).await
    }

    #[tool(description = "List node pools in a Kubernetes cluster")]
    async fn doctl_k8s_node_pool_list(
        &self,
        Parameters(params): Parameters<K8sNodePoolListParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::kubernetes::node_pool_list(params).await
    }

    #[tool(description = "Update a node pool in a Kubernetes cluster (e.g. scale nodes)")]
    async fn doctl_k8s_node_pool_update(
        &self,
        Parameters(params): Parameters<K8sNodePoolUpdateParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::kubernetes::node_pool_update(params).await
    }

    #[tool(description = "Upgrade a Kubernetes cluster to a new version")]
    async fn doctl_k8s_cluster_upgrade(
        &self,
        Parameters(params): Parameters<K8sClusterUpgradeParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::kubernetes::cluster_upgrade(params).await
    }

    // === Apps Platform Tools ===

    #[tool(description = "List all App Platform apps")]
    async fn doctl_apps_list(
        &self,
        Parameters(params): Parameters<AppsListParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::apps::list(params).await
    }

    #[tool(description = "Get details of a specific App Platform app")]
    async fn doctl_apps_get(
        &self,
        Parameters(params): Parameters<AppsGetParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::apps::get(params).await
    }

    #[tool(description = "Create a new App Platform app from a spec file")]
    async fn doctl_apps_create(
        &self,
        Parameters(params): Parameters<AppsCreateParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::apps::create(params).await
    }

    #[tool(description = "Update an existing App Platform app")]
    async fn doctl_apps_update(
        &self,
        Parameters(params): Parameters<AppsUpdateParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::apps::update(params).await
    }

    #[tool(description = "Get logs for an App Platform app")]
    async fn doctl_apps_logs(
        &self,
        Parameters(params): Parameters<AppsLogsParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::apps::logs(params).await
    }

    // === Database Tools ===

    #[tool(description = "List all database clusters")]
    async fn doctl_db_list(
        &self,
        Parameters(params): Parameters<DbListParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::database::list(params).await
    }

    #[tool(description = "Get details of a specific database cluster")]
    async fn doctl_db_get(
        &self,
        Parameters(params): Parameters<DbGetParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::database::get(params).await
    }

    #[tool(description = "Get connection info for a database cluster")]
    async fn doctl_db_connection(
        &self,
        Parameters(params): Parameters<DbConnectionParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::database::connection(params).await
    }

    #[tool(description = "List connection pools for a database cluster")]
    async fn doctl_db_pool_list(
        &self,
        Parameters(params): Parameters<DbPoolListParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::database::pool_list(params).await
    }

    // === Domain Tools ===

    #[tool(description = "List DNS records for a domain")]
    async fn doctl_domain_records_list(
        &self,
        Parameters(params): Parameters<DomainRecordsListParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::domain::records_list(params).await
    }

    #[tool(description = "Create a DNS record for a domain")]
    async fn doctl_domain_records_create(
        &self,
        Parameters(params): Parameters<DomainRecordsCreateParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::domain::records_create(params).await
    }

    // === Account/Utility Tools ===

    #[tool(description = "Get DigitalOcean account information")]
    async fn doctl_account_get(&self) -> Result<CallToolResult, McpError> {
        handlers::account::get().await
    }

    #[tool(description = "Get DigitalOcean account balance and billing info")]
    async fn doctl_balance_get(&self) -> Result<CallToolResult, McpError> {
        handlers::account::balance().await
    }
}

#[tool_handler]
impl rmcp::ServerHandler for DoctlMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "DigitalOcean CLI (doctl) MCP Server - provides tools for managing \
                 DigitalOcean infrastructure including droplets, Kubernetes clusters, \
                 App Platform apps, databases, and DNS records."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

impl Default for DoctlMcpServer {
    fn default() -> Self {
        Self::new()
    }
}
