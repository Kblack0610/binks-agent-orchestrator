//! Main application state and logic

use std::collections::HashSet;
use std::time::Duration;

use iced::widget::{button, column, container, row, scrollable, text, Column};
use iced::{Alignment, Color, Element, Length, Subscription, Task};
use iced::theme::Style;
use iced_layershell::to_layer_message;

use crate::agents::tmux::{TmuxAgent, TmuxMonitor};
use crate::agents::{Agent, AgentState, ProjectGroup};
use crate::ui::theme::CatppuccinTheme;

/// Application state
pub struct App {
    /// Visible state
    visible: bool,
    /// Project groups (agents grouped by project)
    project_groups: Vec<ProjectGroup>,
    /// Expanded project groups (by short_name)
    expanded_projects: HashSet<String>,
    /// Theme colors
    colors: CatppuccinTheme,
    /// Has urgent agents needing attention
    has_urgent: bool,
}

/// Application messages
#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    /// Toggle panel visibility
    Toggle,
    /// Project groups updated
    ProjectGroupsUpdated(Vec<ProjectGroup>),
    /// Toggle expansion of a project group
    ToggleProject(String), // project short_name
    /// Click on agent to open in terminal
    OpenAgent(String, usize), // session, window_index
    /// Tick for polling
    Tick,
    /// Hide panel (after timeout or manual)
    Hide,
    /// Show panel (on urgent or manual)
    Show,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let app = Self {
            visible: true, // Start visible for testing
            project_groups: Vec::new(),
            expanded_projects: HashSet::new(),
            colors: CatppuccinTheme::mocha(),
            has_urgent: false,
        };

        (app, Task::none())
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Toggle => {
                self.visible = !self.visible;
                Task::none()
            }
            Message::Show => {
                self.visible = true;
                Task::none()
            }
            Message::Hide => {
                self.visible = false;
                Task::none()
            }
            Message::ProjectGroupsUpdated(groups) => {
                // Check for urgent agents in any group
                let new_urgent = groups.iter().any(|g| g.has_urgent());

                // Auto-show on urgent
                if new_urgent && !self.has_urgent {
                    self.visible = true;
                }

                self.has_urgent = new_urgent;
                self.project_groups = groups;
                Task::none()
            }
            Message::ToggleProject(short_name) => {
                if self.expanded_projects.contains(&short_name) {
                    self.expanded_projects.remove(&short_name);
                } else {
                    self.expanded_projects.insert(short_name);
                }
                Task::none()
            }
            Message::OpenAgent(session, window_idx) => {
                // Open in kitty terminal
                let cmd = format!(
                    "kitty --single-instance -e tmux attach -t '{}:{}'",
                    session, window_idx
                );
                let _ = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(&cmd)
                    .spawn();
                Task::none()
            }
            Message::Tick => {
                // Fetch grouped agents
                let groups = TmuxMonitor::fetch_grouped_agents();
                Task::done(Message::ProjectGroupsUpdated(groups))
            }
            _ => Task::none(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        if !self.visible {
            // Hidden state - minimal element
            return container(text(""))
                .width(Length::Fixed(1.0))
                .height(Length::Fixed(1.0))
                .into();
        }

        let header = container(
            row![
                text("Agents").size(14),
                iced::widget::Space::new().width(Length::Fill),
                button(text("x").size(12))
                    .on_press(Message::Hide)
                    .padding(2),
            ]
            .align_y(Alignment::Center),
        )
        .padding(8)
        .style(|_theme| container::Style {
            background: Some(Color::from_rgb(0.19, 0.20, 0.27).into()),
            ..Default::default()
        });

        // Project groups list
        let group_rows: Vec<Element<'_, Message>> = self
            .project_groups
            .iter()
            .flat_map(|group| self.view_project_group(group))
            .collect();

        let groups_list: Element<'_, Message> = if group_rows.is_empty() {
            container(text("No agents running").size(12))
                .padding(16)
                .center_x(Length::Fill)
                .into()
        } else {
            scrollable(Column::with_children(group_rows).spacing(2).padding(8)).into()
        };

        // Summary footer
        let summary = self.view_summary();

        container(column![header, groups_list, summary,].spacing(0))
            .width(Length::Fixed(300.0))
            .height(Length::Shrink)
            .style(|_theme| container::Style {
                background: Some(Color::from_rgb(0.12, 0.12, 0.18).into()),
                border: iced::Border {
                    color: Color::from_rgb(0.27, 0.28, 0.35),
                    width: 1.0,
                    radius: 8.0.into(),
                },
                ..Default::default()
            })
            .into()
    }

    /// View a project group (header row + optionally expanded agents)
    fn view_project_group(&self, group: &ProjectGroup) -> Vec<Element<'_, Message>> {
        let mut elements = Vec::new();
        let is_expanded = self.expanded_projects.contains(&group.short_name);

        // Expansion arrow
        let arrow = if is_expanded { "▼" } else { "▶" };

        // Status icons for all agents in group
        let icons = group.status_icons();

        // Count text
        let agent_count = group.agents.len();
        let count_text = format!("{} agent{}", agent_count, if agent_count == 1 { "" } else { "s" });

        // Clone values for the closure and widget tree
        let short_name = group.short_name.clone();

        // Header row - clickable to expand/collapse
        let header = button(
            row![
                text(arrow).size(12),
                text(group.short_name.clone()).size(14),
                text(icons).size(14),
                iced::widget::Space::new().width(Length::Fill),
                text(count_text).size(10),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .on_press(Message::ToggleProject(short_name))
        .padding(8)
        .width(Length::Fill)
        .style(move |_theme, status| {
            let bg = match status {
                button::Status::Hovered => Color::from_rgba(1.0, 1.0, 1.0, 0.08),
                _ => Color::from_rgba(1.0, 1.0, 1.0, 0.03),
            };
            button::Style {
                background: Some(bg.into()),
                border: iced::Border::default(),
                text_color: Color::WHITE,
                ..Default::default()
            }
        });

        elements.push(header.into());

        // If expanded, show individual agents
        if is_expanded {
            for agent in &group.agents {
                elements.push(self.view_agent(agent, true));
            }
        }

        elements
    }

    /// View an individual agent row
    fn view_agent(&self, agent: &TmuxAgent, indented: bool) -> Element<'_, Message> {
        let state_icon = match agent.state() {
            AgentState::Idle => "✓",
            AgentState::Ready => "✓",
            AgentState::Working => "~",
            AgentState::Urgent => "!",
        };

        // Use session:window for display in expanded view
        let display_name = format!("{}:{}", agent.session, agent.window_index);
        let status_text = agent.status_text();
        let time_ago_text = agent.time_ago();

        let name = text(display_name).size(11);
        let status = text(status_text).size(9);
        let time_ago = text(time_ago_text).size(9);

        // Build inner row with optional indentation via horizontal space
        let inner_row = row![
            if indented {
                iced::widget::Space::new().width(Length::Fixed(12.0))
            } else {
                iced::widget::Space::new().width(Length::Fixed(0.0))
            },
            text(state_icon).size(12),
            column![name, status].spacing(1),
            iced::widget::Space::new().width(Length::Fill),
            time_ago,
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        button(inner_row)
        .on_press(Message::OpenAgent(
            agent.session.clone(),
            agent.window_index,
        ))
        .padding(6)
        .width(Length::Fill)
        .style(|_theme, status| {
            let bg = match status {
                button::Status::Hovered => Color::from_rgba(1.0, 1.0, 1.0, 0.05),
                _ => Color::TRANSPARENT,
            };
            button::Style {
                background: Some(bg.into()),
                border: iced::Border::default(),
                text_color: Color::from_rgb(0.8, 0.8, 0.85),
                ..Default::default()
            }
        })
        .into()
    }

    fn view_summary(&self) -> Element<'_, Message> {
        // Count totals across all project groups
        let total: usize = self.project_groups.iter().map(|g| g.agents.len()).sum();
        let working: usize = self
            .project_groups
            .iter()
            .flat_map(|g| &g.agents)
            .filter(|a| a.state() == AgentState::Working)
            .count();
        let urgent: usize = self
            .project_groups
            .iter()
            .flat_map(|g| &g.agents)
            .filter(|a| a.state() == AgentState::Urgent)
            .count();

        let summary_text = format!("{} agents | {} working | {} urgent", total, working, urgent);

        container(text(summary_text).size(10))
            .padding(8)
            .style(|_theme| container::Style {
                background: Some(Color::from_rgb(0.19, 0.20, 0.27).into()),
                ..Default::default()
            })
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        // Poll every 2 seconds
        iced::time::every(Duration::from_secs(2)).map(|_| Message::Tick)
    }

    pub fn style(&self, theme: &iced::Theme) -> Style {
        Style {
            background_color: Color::TRANSPARENT,
            text_color: theme.palette().text,
        }
    }
}
