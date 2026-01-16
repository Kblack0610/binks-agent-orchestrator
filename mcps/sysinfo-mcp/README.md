# System Info MCP Server

A cross-platform MCP (Model Context Protocol) server that provides system information tools. Works on Linux, macOS, and Windows.

## Features

- **OS Info**: Name, version, kernel, hostname, architecture
- **CPU**: Model, vendor, physical/logical cores, frequency, usage
- **Memory**: Total, used, available RAM and swap
- **Disk**: Partitions, mount points, filesystem types, usage
- **Network**: Interfaces, MAC addresses, IP addresses, traffic stats
- **Uptime**: Seconds and human-readable format, boot timestamp

## Tools

| Tool | Description | Parameters |
|------|-------------|------------|
| `get_os_info` | OS name, version, kernel, hostname, arch | None |
| `get_cpu_info` | CPU model, cores, vendor, frequency | `include_per_core: bool` |
| `get_cpu_usage` | Current CPU usage percentage | `per_core: bool` |
| `get_memory_info` | RAM & swap: total, used, available | None |
| `get_disk_info` | Disk partitions and usage | `mount_point: string` (filter) |
| `get_network_interfaces` | Network interfaces with IPs/MACs | `interface: string` (filter) |
| `get_uptime` | Uptime in seconds + human-readable | None |
| `get_system_summary` | Combined summary of all info | None |

## Building

```bash
cd mcps/sysinfo-mcp
cargo build --release
```

## Usage

### Direct

```bash
./target/release/sysinfo-mcp
```

### With `.mcp.json`

```json
{
  "mcpServers": {
    "sysinfo": {
      "command": "./mcps/sysinfo-mcp/target/release/sysinfo-mcp"
    }
  }
}
```

## Example Output

### get_os_info
```json
{
  "name": "CachyOS Linux",
  "version": "rolling",
  "kernel_version": "6.18.5-2-cachyos",
  "hostname": "my-machine",
  "architecture": "x86_64"
}
```

### get_memory_info
```json
{
  "total_bytes": 134124404736,
  "used_bytes": 23719751680,
  "available_bytes": 110404653056,
  "usage_percent": 17.68,
  "swap_total_bytes": 134124400640,
  "swap_used_bytes": 0,
  "swap_usage_percent": 0.0
}
```

## Dependencies

- `rmcp` - MCP SDK for Rust
- `sysinfo` - Cross-platform system information
- `tokio` - Async runtime
- `serde` / `serde_json` - JSON serialization
- `schemars` - JSON Schema generation

## License

MIT
