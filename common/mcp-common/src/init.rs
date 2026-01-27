//! Server initialization utilities
//!
//! Provides standardized tracing setup and the `serve_stdio!` macro
//! for consistent MCP server initialization across all servers.

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize tracing/logging for MCP servers
///
/// Sets up logging to stderr (stdout is reserved for MCP protocol) with:
/// - Formatted output without ANSI colors (for clean logs)
/// - Environment-based filtering via RUST_LOG
/// - Default log level of `info` for the specified crate
///
/// Set `LOG_FORMAT=json` for structured JSON output (useful for production/log aggregation).
/// Default is human-readable text output.
///
/// # Arguments
///
/// * `crate_name` - The name of the MCP server crate (e.g., "sysinfo_mcp")
///
/// # Example
///
/// ```rust,ignore
/// mcp_common::init_tracing("my_mcp");
/// ```
pub fn init_tracing(crate_name: &str) -> anyhow::Result<()> {
    let directive = format!("{}=info", crate_name);
    let filter = EnvFilter::from_default_env().add_directive(directive.parse()?);

    let use_json = std::env::var("LOG_FORMAT")
        .map(|v| v.eq_ignore_ascii_case("json"))
        .unwrap_or(false);

    let registry = tracing_subscriber::registry().with(filter);

    if use_json {
        registry
            .with(
                tracing_subscriber::fmt::layer()
                    .json()
                    .with_writer(std::io::stderr),
            )
            .init();
    } else {
        registry
            .with(
                tracing_subscriber::fmt::layer()
                    .with_writer(std::io::stderr)
                    .with_ansi(false),
            )
            .init();
    }

    Ok(())
}

/// Macro for standardized MCP server initialization
///
/// This macro replaces ~30 lines of boilerplate in each MCP server's `main.rs`
/// with a single line. It handles:
///
/// - Tracing/logging initialization (to stderr)
/// - Server instantiation
/// - stdio transport setup
/// - Graceful shutdown
///
/// # Arguments
///
/// * `$server_type` - The server struct type (must implement `Default` or `new()`)
/// * `$crate_name` - String literal for the crate name (used in logging)
///
/// # Example
///
/// ```rust,ignore
/// use mcp_common::serve_stdio;
///
/// mod server;
/// use server::MyMcpServer;
///
/// serve_stdio!(MyMcpServer, "my_mcp");
/// ```
///
/// This expands to a complete `#[tokio::main] async fn main()` that:
/// 1. Initializes tracing to stderr
/// 2. Creates the server with `::new()`
/// 3. Serves via stdio transport
/// 4. Waits for shutdown
#[macro_export]
macro_rules! serve_stdio {
    ($server_type:ty, $crate_name:expr) => {
        #[tokio::main]
        async fn main() -> anyhow::Result<()> {
            use rmcp::ServiceExt;

            $crate::init_tracing($crate_name)?;

            tracing::info!(concat!("Starting ", $crate_name, " MCP Server"));

            let server = <$server_type>::new();
            let service = server.serve(rmcp::transport::stdio()).await?;

            tracing::info!("Server running, waiting for requests...");

            service.waiting().await?;

            tracing::info!("Server shutting down");
            Ok(())
        }
    };
}

#[cfg(test)]
mod tests {
    // Note: Can't easily test tracing initialization in unit tests
    // as it can only be initialized once per process
}
