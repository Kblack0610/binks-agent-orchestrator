//! Authentication middleware for the web server
//!
//! Provides bearer token authentication for API endpoints.
//! Auth is optional - if BINKS_API_TOKEN is not set, all requests are allowed.

use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::OnceLock;

/// Static storage for the API token
static API_TOKEN: OnceLock<Option<String>> = OnceLock::new();

/// Initialize the API token from environment
pub fn init_auth() {
    API_TOKEN.get_or_init(|| std::env::var("BINKS_API_TOKEN").ok());
}

/// Get the configured API token (if any)
fn get_api_token() -> Option<&'static str> {
    API_TOKEN.get().and_then(|t| t.as_deref())
}

/// Check if authentication is enabled
pub fn is_auth_enabled() -> bool {
    get_api_token().is_some()
}

/// Authentication middleware
///
/// If BINKS_API_TOKEN is set, validates the Authorization header.
/// If not set, allows all requests (backwards compatible).
pub async fn auth_middleware(request: Request, next: Next) -> Result<Response, StatusCode> {
    // If no token configured, allow all requests
    let Some(expected_token) = get_api_token() else {
        return Ok(next.run(request).await);
    };

    // Extract and validate the Authorization header
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            let token = &header[7..]; // Skip "Bearer "
            if token == expected_token {
                Ok(next.run(request).await)
            } else {
                tracing::warn!(
                    "Invalid bearer token provided for {}",
                    request.uri().path()
                );
                Err(StatusCode::UNAUTHORIZED)
            }
        }
        Some(_) => {
            tracing::warn!(
                "Invalid Authorization header format for {}",
                request.uri().path()
            );
            Err(StatusCode::UNAUTHORIZED)
        }
        None => {
            tracing::warn!(
                "Missing Authorization header for {}",
                request.uri().path()
            );
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_disabled_by_default() {
        // Reset for test
        init_auth();
        // Without BINKS_API_TOKEN set, auth should be disabled
        // (depends on test environment)
    }
}
