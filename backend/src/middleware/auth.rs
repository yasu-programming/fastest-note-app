use crate::services::AuthService;
use axum::{
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::{IntoResponse, Json, Response},
};
use serde_json::json;
use std::sync::Arc;

pub async fn auth_middleware(
    State(auth_service): State<Arc<AuthService>>,
    mut req: Request,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    // Extract token from Authorization header
    let token = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .and_then(|header| header.strip_prefix("Bearer "));

    let token = match token {
        Some(token) => token,
        None => {
            return Ok((
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "success": false,
                    "error": "Authorization header missing or invalid"
                })),
            )
                .into_response());
        }
    };

    // Validate token and get user
    let user = match auth_service.get_user_by_token(token).await {
        Ok(user) => user,
        Err(e) => {
            let error_msg = e.to_string();
            let status = if error_msg.contains("Invalid token") || 
                         error_msg.contains("revoked") ||
                         error_msg.contains("expired") {
                StatusCode::UNAUTHORIZED
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };

            return Ok((
                status,
                Json(json!({
                    "success": false,
                    "error": format!("Authentication failed: {}", error_msg)
                })),
            )
                .into_response());
        }
    };

    // Add user to request extensions
    req.extensions_mut().insert(user);

    Ok(next.run(req).await)
}

pub async fn optional_auth_middleware(
    State(auth_service): State<Arc<AuthService>>,
    mut req: Request,
    next: Next,
) -> Response {
    // Extract token from Authorization header (optional)
    if let Some(token) = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .and_then(|header| header.strip_prefix("Bearer "))
    {
        // If token is present, validate it
        if let Ok(user) = auth_service.get_user_by_token(token).await {
            req.extensions_mut().insert(user);
        }
        // If token is invalid, we continue without authentication
        // The handler can check if user is present in extensions
    }

    next.run(req).await
}

pub async fn admin_auth_middleware(
    State(auth_service): State<Arc<AuthService>>,
    mut req: Request,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    // First run normal auth middleware logic
    let token = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .and_then(|header| header.strip_prefix("Bearer "));

    let token = match token {
        Some(token) => token,
        None => {
            return Ok((
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "success": false,
                    "error": "Authorization header missing"
                })),
            )
                .into_response());
        }
    };

    let user = match auth_service.get_user_by_token(token).await {
        Ok(user) => user,
        Err(e) => {
            return Ok((
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "success": false,
                    "error": format!("Authentication failed: {}", e)
                })),
            )
                .into_response());
        }
    };

    // Check if user is admin (in a real app, you'd have role-based permissions)
    // For now, we'll assume admin status is determined by email domain or specific user IDs
    let is_admin = user.email.ends_with("@admin.com") || user.id == 1;
    
    if !is_admin {
        return Ok((
            StatusCode::FORBIDDEN,
            Json(json!({
                "success": false,
                "error": "Admin privileges required"
            })),
        )
            .into_response());
    }

    req.extensions_mut().insert(user);
    Ok(next.run(req).await)
}

// Helper function to extract user from request extensions
pub fn get_user_from_request(req: &Request) -> Option<&crate::models::User> {
    req.extensions().get::<crate::models::User>()
}

// Middleware for WebSocket authentication (token from query param)
pub async fn websocket_auth_middleware(
    State(auth_service): State<Arc<AuthService>>,
    mut req: Request,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    // Extract token from query parameter for WebSocket connections
    let token = req
        .uri()
        .query()
        .and_then(|query| {
            url::form_urlencoded::parse(query.as_bytes())
                .find(|(key, _)| key == "token")
                .map(|(_, value)| value.into_owned())
        });

    let token = match token {
        Some(token) => token,
        None => {
            return Ok((
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "success": false,
                    "error": "Authentication token required in query parameter"
                })),
            )
                .into_response());
        }
    };

    let user = match auth_service.get_user_by_token(&token).await {
        Ok(user) => user,
        Err(e) => {
            return Ok((
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "success": false,
                    "error": format!("Authentication failed: {}", e)
                })),
            )
                .into_response());
        }
    };

    req.extensions_mut().insert(user);
    Ok(next.run(req).await)
}