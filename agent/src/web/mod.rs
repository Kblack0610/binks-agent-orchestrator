//! Web server module for the chat interface
//!
//! Provides an HTTP server with REST API and WebSocket support for the chat UI.

pub mod api;
pub mod state;
pub mod ws;

use anyhow::Result;
use axum::{
    extract::State,
    http::{header, StatusCode, Uri},
    response::{Html, IntoResponse, Response},
    routing::{delete, get, patch, post},
    Router,
};
use rust_embed::RustEmbed;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};

use crate::db::Database;
use state::AppState;

/// Embedded static files for the frontend (production mode)
#[derive(RustEmbed)]
#[folder = "static/"]
struct StaticAssets;

/// Configuration for the web server
pub struct WebConfig {
    pub port: u16,
    pub ollama_url: String,
    pub model: String,
    pub system_prompt: Option<String>,
    pub dev_mode: bool,
}

/// Start the web server
pub async fn serve(config: WebConfig) -> Result<()> {
    // Initialize database
    let db = Database::open()?;
    tracing::info!("Database opened at {:?}", Database::default_path()?);

    // Create app state
    let state = AppState::new(
        db,
        config.ollama_url.clone(),
        config.model.clone(),
        config.system_prompt.clone(),
    )?;

    // Build router
    let app = create_router(state, config.dev_mode);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("Starting web server on http://localhost:{}", config.port);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Create the router with all routes
fn create_router(state: AppState, dev_mode: bool) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api_routes = Router::new()
        // Conversations
        .route("/conversations", get(api::list_conversations))
        .route("/conversations", post(api::create_conversation))
        .route("/conversations/{id}", get(api::get_conversation))
        .route("/conversations/{id}", patch(api::update_conversation))
        .route("/conversations/{id}", delete(api::delete_conversation))
        // Messages
        .route("/conversations/{id}/messages", get(api::get_messages))
        // Tools
        .route("/tools", get(api::list_tools))
        // Health
        .route("/health", get(api::health_check));

    let ws_routes = Router::new()
        .route("/chat/{conversation_id}", get(ws::chat_handler));

    let mut router = Router::new()
        .nest("/api", api_routes)
        .nest("/ws", ws_routes)
        .layer(cors)
        .with_state(state);

    // Static file serving
    if dev_mode {
        // In dev mode, we'll proxy to Vite dev server or serve a placeholder
        router = router.fallback(dev_fallback);
    } else {
        // In production, serve embedded static files
        router = router.fallback(static_handler);
    }

    router
}

/// Serve embedded static files (production mode)
async fn static_handler(uri: Uri) -> Response<axum::body::Body> {
    let path = uri.path().trim_start_matches('/');

    // Try the exact path first
    if let Some(content) = StaticAssets::get(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        let body = axum::body::Body::from(content.data.to_vec());
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime.as_ref())
            .body(body)
            .unwrap();
    }

    // For SPA routing, serve index.html for non-file paths
    if !path.contains('.') || path.is_empty() {
        if let Some(content) = StaticAssets::get("index.html") {
            let body = axum::body::Body::from(content.data.to_vec());
            return Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/html")
                .body(body)
                .unwrap();
        }
    }

    // 404 for missing files
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(axum::body::Body::from("Not Found"))
        .unwrap()
}

/// Dev mode fallback - shows instructions
async fn dev_fallback() -> Html<&'static str> {
    Html(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>Binks Agent - Dev Mode</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
            max-width: 600px;
            margin: 100px auto;
            padding: 20px;
            background: #1a1a2e;
            color: #eee;
        }
        h1 { color: #22c55e; }
        code {
            background: #16213e;
            padding: 2px 8px;
            border-radius: 4px;
        }
        pre {
            background: #16213e;
            padding: 16px;
            border-radius: 8px;
            overflow-x: auto;
        }
        a { color: #22c55e; }
    </style>
</head>
<body>
    <h1>Binks Agent - Dev Mode</h1>
    <p>The API server is running. To connect the frontend:</p>
    <pre>cd /path/to/platform/apps/binks-chat
pnpm dev</pre>
    <p>Then visit <a href="http://localhost:5173">http://localhost:5173</a></p>
    <h2>API Endpoints</h2>
    <ul>
        <li><code>GET /api/health</code> - Health check</li>
        <li><code>GET /api/conversations</code> - List conversations</li>
        <li><code>POST /api/conversations</code> - Create conversation</li>
        <li><code>GET /api/conversations/:id</code> - Get conversation with messages</li>
        <li><code>PATCH /api/conversations/:id</code> - Update conversation</li>
        <li><code>DELETE /api/conversations/:id</code> - Delete conversation</li>
        <li><code>GET /api/tools</code> - List available tools</li>
        <li><code>WS /ws/chat/:id</code> - WebSocket for chat</li>
    </ul>
</body>
</html>"#,
    )
}
