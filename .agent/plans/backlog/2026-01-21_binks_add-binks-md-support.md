# Add BINKS.md System Prompt File Support

**Status:** Backlog
**Priority:** Low
**Effort:** Small (~50 lines total)

## Goal
Add CLAUDE.md-like functionality to binks - automatically read markdown instruction files and prepend them to the system prompt.

## Scope
- **Small, isolated change** - new module + minor main.rs modification
- **No changes to core agent logic** (Agent struct, chat loop, config.rs, context.rs)
- **Additive only** - existing behavior unchanged when no BINKS.md exists

## File Locations (Priority Order)
1. `~/.config/binks/BINKS.md` - Global instructions (user-wide)
2. `./.agent/BINKS.md` - Project-specific instructions
3. `./BINKS.md` - Project root fallback

All found files are concatenated (global first, then project).

## Implementation

### 1. Create `agent/src/binks.rs` (~40 lines)

```rust
//! BINKS.md instruction file loader
use std::path::PathBuf;
use std::fs;

/// Loads and concatenates BINKS.md content from standard locations
pub fn load_binks_content() -> Option<String> {
    let mut contents = Vec::new();

    // Global: ~/.config/binks/BINKS.md
    if let Some(home) = dirs::home_dir() {
        let global = home.join(".config/binks/BINKS.md");
        if let Ok(content) = fs::read_to_string(&global) {
            contents.push(content);
        }
    }

    // Project: ./.agent/BINKS.md or ./BINKS.md
    let project_paths = [
        PathBuf::from(".agent/BINKS.md"),
        PathBuf::from("BINKS.md"),
    ];

    for path in &project_paths {
        if let Ok(content) = fs::read_to_string(path) {
            contents.push(content);
            break; // Only use first found project file
        }
    }

    if contents.is_empty() {
        None
    } else {
        Some(contents.join("\n\n---\n\n"))
    }
}
```

### 2. Modify `agent/src/lib.rs`
Add module declaration:
```rust
pub mod binks;
```

### 3. Modify `agent/src/main.rs` (~5 lines changed)
Around line 408, change:
```rust
// Before
let system_prompt = system
    .or_else(|| file_config.agent.system_prompt.clone())
    .or_else(|| {
        let ctx = EnvironmentContext::gather();
        Some(ctx.to_system_prompt())
    });

// After
let base_prompt = system
    .or_else(|| file_config.agent.system_prompt.clone())
    .or_else(|| {
        let ctx = EnvironmentContext::gather();
        Some(ctx.to_system_prompt())
    });

let system_prompt = match (agent::binks::load_binks_content(), base_prompt) {
    (Some(binks), Some(base)) => Some(format!("{}\n\n{}", binks, base)),
    (Some(binks), None) => Some(binks),
    (None, base) => base,
};
```

### 4. Add `dirs` dependency (if not present)
Check `agent/Cargo.toml` - may already have it for home directory resolution.

## Files Modified
- `agent/src/binks.rs` - **NEW** (~40 lines)
- `agent/src/lib.rs` - Add 1 line (`pub mod binks;`)
- `agent/src/main.rs` - Change ~5 lines around line 408

## Verification
1. `cargo build` - Ensure it compiles
2. `cargo test` - Existing tests pass
3. Manual test:
   ```bash
   # Create test file
   echo "Always check .agent/plans/ before implementing." > .agent/BINKS.md

   # Run agent and verify prompt includes the content
   agent agent "what are your instructions?"

   # Clean up
   rm .agent/BINKS.md
   ```

## Future Enhancements (Out of Scope)
- Workspace-level BINKS.md (parent directories)
- `--no-binks` flag to disable
- Verbose logging of loaded files
