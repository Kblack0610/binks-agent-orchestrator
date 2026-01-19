//! Agent monitoring modules

pub mod tmux;

use tmux::TmuxAgent;

/// Agent state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    /// Not recently active
    Idle,
    /// At prompt, ready for input
    Ready,
    /// Currently executing/outputting
    Working,
    /// Needs user attention (Y/n prompt, permission request)
    Urgent,
}

/// Common agent trait
pub trait Agent {
    /// Get current state
    fn state(&self) -> AgentState;

    /// Get display name
    fn display_name(&self) -> String;

    /// Get status text
    fn status_text(&self) -> String;

    /// Get time since last activity
    fn time_ago(&self) -> String;
}

/// A project group containing one or more agents
#[derive(Debug, Clone)]
pub struct ProjectGroup {
    /// Full project name (from directory)
    pub name: String,
    /// 3-char abbreviation for display
    pub short_name: String,
    /// Agents belonging to this project
    pub agents: Vec<TmuxAgent>,
}

impl ProjectGroup {
    /// Get the highest priority state among all agents
    pub fn overall_state(&self) -> AgentState {
        if self.agents.iter().any(|a| a.state == AgentState::Urgent) {
            AgentState::Urgent
        } else if self.agents.iter().any(|a| a.state == AgentState::Working) {
            AgentState::Working
        } else if self.agents.iter().any(|a| a.state == AgentState::Ready) {
            AgentState::Ready
        } else {
            AgentState::Idle
        }
    }

    /// Get status icons string (one per agent)
    /// Returns icons like "✓~!" for ready, working, urgent
    pub fn status_icons(&self) -> String {
        self.agents
            .iter()
            .map(|a| match a.state {
                AgentState::Idle => "✓",
                AgentState::Ready => "✓",
                AgentState::Working => "~",
                AgentState::Urgent => "!",
            })
            .collect::<Vec<_>>()
            .join("")
    }

    /// Check if any agent is urgent
    pub fn has_urgent(&self) -> bool {
        self.agents.iter().any(|a| a.state == AgentState::Urgent)
    }

    /// Check if any agent is working
    pub fn has_working(&self) -> bool {
        self.agents.iter().any(|a| a.state == AgentState::Working)
    }
}
