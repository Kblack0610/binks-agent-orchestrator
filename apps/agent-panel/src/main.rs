//! Agent Panel - System-level agent monitoring for Hyprland
//!
//! A Rust-based floating panel using wlr-layer-shell to monitor:
//! - AI/LLM agents (Claude, Aider, OpenCode) via tmux
//! - System processes
//! - Workflow agents (future)
//! - OpenTelemetry traces (future)

mod agents;
mod app;
mod config;
mod ipc;
mod ui;

use iced::theme::Style;
use iced_layershell::reexport::{Anchor, KeyboardInteractivity};
use iced_layershell::settings::LayerShellSettings;
use iced_layershell::application;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use app::{App, Message};

fn main() -> iced_layershell::Result {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "agent_panel=info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting agent-panel");

    application(App::new, namespace, update, view)
        .subscription(subscription)
        .style(style)
        .layer_settings(LayerShellSettings {
            size: Some((320, 200)),
            anchor: Anchor::Top | Anchor::Right,
            margin: (10, 10, 0, 0),
            exclusive_zone: 0,
            keyboard_interactivity: KeyboardInteractivity::None,
            events_transparent: true,
            ..Default::default()
        })
        .run()
}

fn namespace() -> String {
    String::from("agent-panel")
}

fn update(app: &mut App, message: Message) -> iced::Task<Message> {
    app.update(message)
}

fn view(app: &App) -> iced::Element<'_, Message> {
    app.view()
}

fn subscription(app: &App) -> iced::Subscription<Message> {
    app.subscription()
}

fn style(app: &App, theme: &iced::Theme) -> Style {
    app.style(theme)
}
