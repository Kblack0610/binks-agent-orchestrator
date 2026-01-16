# System Information MCP Server

**Goal:** Create a cross-platform, modular MCP server in Rust that provides system information tools.

## Requirements
- Cross-platform: Linux, macOS, Windows
- Modular: Start with essentials, easy to extend
- Rust with rmcp 0.13 (matches existing github-gh pattern)

## Project Structure

```
mcps/sysinfo-mcp/
├── Cargo.toml
├── README.md
└── src/
    ├── main.rs           # Entry point, stdio transport
    ├── server.rs         # MCP server with tool definitions
    ├── error.rs          # Error types
    ├── info/             # System info collection
    │   ├── mod.rs
    │   ├── os.rs         # OS name, version, kernel, arch
    │   ├── cpu.rs        # CPU model, cores, usage
    │   ├── memory.rs     # RAM total, used, available, swap
    │   ├── disk.rs       # Partitions, mount points, usage
    │   ├── network.rs    # Interfaces, IPs, MACs
    │   └── uptime.rs     # System uptime
    └── types/            # Response structs for JSON
        ├── mod.rs
        ├── os.rs
        ├── cpu.rs
        ├── memory.rs
        ├── disk.rs
        ├── network.rs
        └── uptime.rs
```

## Tools to Implement

| Tool | Description | Parameters |
|------|-------------|------------|
| `get_os_info` | OS name, version, kernel, hostname, arch | None |
| `get_cpu_info` | CPU model, cores, vendor, frequency | `include_per_core: Option<bool>` |
| `get_cpu_usage` | Current CPU usage percentage | `per_core: Option<bool>` |
| `get_memory_info` | RAM & swap: total, used, available | None |
| `get_disk_info` | Disk partitions and usage | `mount_point: Option<String>` |
| `get_network_interfaces` | Network interfaces with IPs/MACs | `interface: Option<String>` |
| `get_uptime` | Uptime in seconds + human-readable | None |
| `get_system_summary` | Combined summary of all info | None |

## Key Dependencies

```toml
rmcp = { version = "0.13", features = ["server", "macros", "transport-io"] }
sysinfo = "0.32"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
schemars = "1.2"
thiserror = "2"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

## Implementation Steps

### 1. Create project structure
- [ ] Create `mcps/sysinfo-mcp/` directory
- [ ] Initialize Cargo.toml with dependencies
- [ ] Create module structure (info/, types/)

### 2. Implement types (src/types/)
- [ ] `OsInfo` struct
- [ ] `CpuInfo`, `CpuCore`, `CpuUsage`, `CoreUsage` structs
- [ ] `MemoryInfo` struct
- [ ] `DiskInfo`, `Partition` structs
- [ ] `NetworkInfo`, `NetworkInterface` structs
- [ ] `UptimeInfo` struct
- [ ] `SystemSummary` struct (combines all)

### 3. Implement info collectors (src/info/)
- [ ] `os.rs` - uses `sysinfo::System` static methods
- [ ] `cpu.rs` - uses `sys.cpus()` and `sys.global_cpu_usage()`
- [ ] `memory.rs` - uses `sys.total_memory()`, `sys.used_memory()`, etc.
- [ ] `disk.rs` - uses `sysinfo::Disks::new_with_refreshed_list()`
- [ ] `network.rs` - uses `sysinfo::Networks::new_with_refreshed_list()`
- [ ] `uptime.rs` - uses `System::uptime()` and `System::boot_time()`

### 4. Implement server (src/server.rs)
- [ ] Create `SysInfoMcpServer` struct with `Arc<Mutex<System>>`
- [ ] Implement `#[tool_router]` with all 8 tools
- [ ] Implement `#[tool_handler]` for `ServerHandler` trait
- [ ] Add parameter structs with `JsonSchema` derive

### 5. Implement entry point (src/main.rs)
- [ ] Setup tracing to stderr
- [ ] Create server instance
- [ ] Serve via stdio transport

### 6. Build and integrate
- [ ] Build release binary
- [ ] Add to `.mcp.json` configuration
- [ ] Test with agent

## Reference Files
- `mcps/github-gh/src/server.rs` - tool_router pattern
- `mcps/github-gh/src/main.rs` - entry point pattern
- `mcps/github-gh/Cargo.toml` - dependency versions

## Verification

1. **Build test:**
   ```bash
   cd mcps/sysinfo-mcp && cargo build --release
   ```

2. **Manual tool test:**
   ```bash
   echo '{"jsonrpc":"2.0","method":"tools/list","id":1}' | ./target/release/sysinfo-mcp
   ```

3. **Integration test:** Add to `.mcp.json` and verify tools appear in agent:
   ```json
   {
     "mcpServers": {
       "sysinfo": {
         "command": "./mcps/sysinfo-mcp/target/release/sysinfo-mcp"
       }
     }
   }
   ```

4. **Agent test:** Run agent and ask it to get system info - verify it calls the tools correctly.
