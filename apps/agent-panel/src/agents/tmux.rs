//! Tmux-based agent monitoring
//!
//! Monitors tmux sessions for AI agent processes (Claude, Aider, OpenCode)

use std::collections::HashMap;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::agents::{Agent, AgentState, ProjectGroup};

/// A tmux-based AI agent
#[derive(Debug, Clone)]
pub struct TmuxAgent {
    pub session: String,
    pub window_index: usize,
    pub window_name: String,
    pub command: String,
    pub pane_path: String,
    pub state: AgentState,
    pub last_activity: u64, // Unix timestamp
}

impl Agent for TmuxAgent {
    fn state(&self) -> AgentState {
        self.state
    }

    fn display_name(&self) -> String {
        let short_session = match self.session.as_str() {
            "placemyparents" => "pmp",
            "ai-lab" => "lab",
            "dotfiles" => "dot",
            s if s.len() > 6 => &s[..6],
            s => s,
        };
        format!("{}/{}", short_session, self.window_name)
    }

    fn status_text(&self) -> String {
        match self.state {
            AgentState::Idle => "Idle".to_string(),
            AgentState::Ready => "Ready".to_string(),
            AgentState::Working => "Working...".to_string(),
            AgentState::Urgent => "Needs input".to_string(),
        }
    }

    fn time_ago(&self) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let diff = now.saturating_sub(self.last_activity);

        if diff < 5 {
            "now".to_string()
        } else if diff < 60 {
            format!("{}s", diff)
        } else if diff < 3600 {
            format!("{}m", diff / 60)
        } else {
            format!("{}h", diff / 3600)
        }
    }
}

impl TmuxAgent {
    /// Extract project name from pane_path (working directory)
    /// Strips `-agent-N` suffixes from directory names
    pub fn project_name(&self) -> String {
        // Get basename of path
        let dir_name = std::path::Path::new(&self.pane_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| self.pane_path.clone());

        // Strip agent suffixes like -agent-2, -agent2, etc.
        let re = regex::Regex::new(r"-agent-?\d*$").unwrap();
        re.replace(&dir_name, "").to_string()
    }

    /// Get 3-character abbreviation for the project
    pub fn project_short(&self) -> String {
        let name = self.project_name();
        match name.as_str() {
            n if n.starts_with("gheeggle") => "ghe".to_string(),
            "shack" => "shk".to_string(),
            "dotfiles" | ".dotfiles" => "dot".to_string(),
            n if n.starts_with("binks") => "bnk".to_string(),
            "placemyparents" => "pmp".to_string(),
            "ai-lab" => "lab".to_string(),
            other => {
                // Take first 3 chars
                other.chars().take(3).collect()
            }
        }
    }

    /// Get the session:window identifier for opening this agent
    pub fn window_target(&self) -> String {
        format!("{}:{}", self.session, self.window_index)
    }
}

/// Tmux monitor - fetches agent state from tmux
pub struct TmuxMonitor;

impl TmuxMonitor {
    /// Agent command patterns to match
    const AGENT_PATTERNS: &'static [&'static str] = &["claude", "claude-real", "aider", "opencode"];

    /// Fetch all tmux agents
    pub fn fetch_agents() -> Vec<TmuxAgent> {
        let mut agents = Vec::new();
        let mut seen_windows: HashMap<String, bool> = HashMap::new();

        // Get all panes with format: session:window_idx:window_name:command:pid:path
        let output = Command::new("tmux")
            .args([
                "list-panes",
                "-a",
                "-F",
                "#{session_name}:#{window_index}:#{window_name}:#{pane_current_command}:#{pane_pid}:#{pane_current_path}",
            ])
            .output();

        let output = match output {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
            _ => return agents,
        };

        for line in output.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() < 6 {
                continue;
            }

            let session = parts[0];
            let window_idx: usize = parts[1].parse().unwrap_or(0);
            let window_name = parts[2];
            let command = parts[3];
            let pane_path = parts[5];

            // Check if this is an agent command
            if !Self::AGENT_PATTERNS
                .iter()
                .any(|p| command.eq_ignore_ascii_case(p))
            {
                continue;
            }

            // Skip already seen windows
            let window_key = format!("{}:{}", session, window_idx);
            if seen_windows.contains_key(&window_key) {
                continue;
            }
            seen_windows.insert(window_key, true);

            // Get window activity timestamp
            let last_activity = Self::get_window_activity(session, window_idx);

            // Capture pane content to detect state
            let state = Self::detect_state(session, window_idx, last_activity);

            agents.push(TmuxAgent {
                session: session.to_string(),
                window_index: window_idx,
                window_name: window_name.to_string(),
                command: command.to_string(),
                pane_path: pane_path.to_string(),
                state,
                last_activity,
            });
        }

        agents
    }

    /// Fetch agents grouped by project
    pub fn fetch_grouped_agents() -> Vec<ProjectGroup> {
        let agents = Self::fetch_agents();
        let mut groups: HashMap<String, Vec<TmuxAgent>> = HashMap::new();

        // Group agents by project short name
        for agent in agents {
            let short = agent.project_short();
            groups.entry(short).or_default().push(agent);
        }

        // Convert to ProjectGroup vec and sort by name
        let mut result: Vec<ProjectGroup> = groups
            .into_iter()
            .map(|(short_name, agents)| {
                // Get full project name from first agent
                let name = agents
                    .first()
                    .map(|a| a.project_name())
                    .unwrap_or_else(|| short_name.clone());

                ProjectGroup {
                    name,
                    short_name,
                    agents,
                }
            })
            .collect();

        // Sort alphabetically by short name
        result.sort_by_key(|a| a.short_name.clone());

        result
    }

    /// Get window's last activity timestamp
    fn get_window_activity(session: &str, window_idx: usize) -> u64 {
        let output = Command::new("tmux")
            .args([
                "display-message",
                "-p",
                "-t",
                &format!("{}:{}", session, window_idx),
                "#{window_activity}",
            ])
            .output();

        match output {
            Ok(o) if o.status.success() => {
                String::from_utf8_lossy(&o.stdout)
                    .trim()
                    .parse()
                    .unwrap_or(0)
            }
            _ => 0,
        }
    }

    /// Detect agent state from pane content
    fn detect_state(session: &str, window_idx: usize, last_activity: u64) -> AgentState {
        // Capture last 15 lines of pane content
        let output = Command::new("tmux")
            .args([
                "capture-pane",
                "-t",
                &format!("{}:{}", session, window_idx),
                "-p",
                "-S",
                "-15",
            ])
            .output();

        let content = match output {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
            _ => return AgentState::Idle,
        };

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let activity_diff = now.saturating_sub(last_activity);

        // Priority 1: Check for interactive prompts needing input
        if content.contains("[Y/n]")
            || content.contains("[y/N]")
            || content.contains("Allow")
            || content.contains("Deny")
            || content.contains("Do you want to")
        {
            return AgentState::Urgent;
        }

        // Priority 2: Actively working (recent output)
        if activity_diff < 3 {
            return AgentState::Working;
        }

        // Priority 3: At prompt or idle
        if content.contains("> ")
            || content.contains("❯ ")
            || content.contains("⏵⏵")
            || content.contains("Context left until")
        {
            return AgentState::Ready;
        }

        // Fallback: No recent activity = idle
        if activity_diff > 10 {
            AgentState::Idle
        } else {
            AgentState::Working
        }
    }
}
