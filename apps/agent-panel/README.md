# Agent Panel

System-level agent monitoring panel for Hyprland using Wayland layer-shell.

## Features

- **Tmux Agent Monitoring** - Tracks Claude, Aider, OpenCode agents running in tmux
- **Auto-Popup on Urgent** - Panel appears when an agent needs input (Y/n prompts)
- **Click to Attach** - Click any agent row to open its tmux session in kitty
- **State Indicators**:
  - `○` Idle - No recent activity
  - `●` Ready - At prompt, waiting for input
  - `◐` Working - Currently executing
  - `` Urgent - Needs user attention
- **Catppuccin Mocha Theme** - Matches Waybar styling

## Quick Start

```bash
# Build
cd ~/.local/src/agent-panel
cargo build --release

# Run
./target/release/agent-panel
```

## Hyprland Integration

Add to `~/.config/hypr/hyprland.conf`:

```bash
# Auto-start panel
exec-once = ~/.local/src/agent-panel/target/release/agent-panel

# Toggle keybind (future - requires IPC)
# bind = $mainMod, A, exec, agent-panel-ctl toggle
```

## Architecture

```
┌─────────────────────────────────────┐
│  Agent Panel (layer: TOP)           │
│  Anchor: top-right, 320x200px       │
├─────────────────────────────────────┤
│ ● pmp/claude    Ready      now      │
│ ◐ lab/aider     Working    3s       │
│  dot/claude    Needs input         │
├─────────────────────────────────────┤
│ 3 agents | 1 working | 1 urgent     │
└─────────────────────────────────────┘
```

**Stack:**
- `iced` (0.14) - Rust GUI framework
- `iced_layershell` (0.14) - Wayland layer-shell integration
- `tokio` - Async runtime for polling
- `sysinfo` - System process monitoring (planned)

**Data Flow:**
1. Poll tmux every 2s via `tmux list-panes`
2. Capture pane content to detect agent state
3. Check for Y/n prompts → Urgent state
4. Auto-show panel on urgent transition

## Configuration

Edit `config/agents.toml`:

```toml
poll_interval = 2              # Seconds between tmux polls
auto_hide_after = 0            # Auto-hide delay (0 = never)
terminal = "kitty --single-instance -e"
agent_patterns = ["claude", "claude-real", "aider", "opencode"]
```

## Project Structure

```
agent-panel/
├── Cargo.toml
├── config/
│   └── agents.toml       # Configuration
└── src/
    ├── main.rs           # Entry point, layer-shell setup
    ├── app.rs            # Application state & UI
    ├── config.rs         # Config parsing
    ├── ipc.rs            # IPC toggle (planned)
    ├── agents/
    │   ├── mod.rs        # Agent trait
    │   └── tmux.rs       # Tmux session monitoring
    └── ui/
        ├── mod.rs
        └── theme.rs      # Catppuccin colors
```

## Roadmap

- [ ] System process monitor (`sysinfo` crate)
- [ ] IPC toggle via unix socket (for `Super+A` keybind)
- [ ] Workflow agent polling (n8n, Temporal)
- [ ] OpenTelemetry trace receiver
- [ ] Configurable position/size
- [ ] Multiple monitor support

## Dependencies

- Rust 1.70+
- Wayland compositor with layer-shell support (Hyprland, Sway)
- tmux (for agent monitoring)
- kitty (for opening sessions, configurable)

## License

MIT
