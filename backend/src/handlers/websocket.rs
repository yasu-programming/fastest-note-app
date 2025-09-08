use crate::services::{AuthService, WebSocketService};
use axum::{
    extract::{Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct WebSocketQuery {
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct WebSocketStats {
    pub connected_users: usize,
    pub total_connections: usize,
    pub active_senders: usize,
    pub users_with_multiple_connections: usize,
}

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<WebSocketQuery>,
    State(websocket_service): State<Arc<WebSocketService>>,
    State(auth_service): State<Arc<AuthService>>,
) -> Result<Response, (StatusCode, Json<Value>)> {
    // Validate the authentication token
    let user = match auth_service.get_user_by_token(&query.token).await {
        Ok(user) => user,
        Err(e) => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "success": false,
                    "error": format!("Authentication failed: {}", e)
                })),
            ));
        }
    };

    // Handle the WebSocket upgrade
    Ok(websocket_service
        .handle_websocket_upgrade(ws, Query(query), user.id)
        .await)
}

pub async fn get_websocket_stats(
    State(websocket_service): State<Arc<WebSocketService>>,
    State(auth_service): State<Arc<AuthService>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let stats = websocket_service.get_stats().await;
    
    Ok(Json(json!({
        "success": true,
        "data": stats
    })))
}

pub async fn get_connected_users(
    State(websocket_service): State<Arc<WebSocketService>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let connected_users = websocket_service.get_connected_users().await;
    
    Ok(Json(json!({
        "success": true,
        "data": {
            "connected_users": connected_users,
            "count": connected_users.len()
        }
    })))
}

pub async fn check_user_connection(
    State(websocket_service): State<Arc<WebSocketService>>,
    axum::extract::Path(user_id): axum::extract::Path<i32>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let is_connected = websocket_service.is_user_connected(user_id).await;
    let connection_count = websocket_service.get_user_connection_count(user_id).await;
    
    Ok(Json(json!({
        "success": true,
        "data": {
            "user_id": user_id,
            "is_connected": is_connected,
            "connection_count": connection_count
        }
    })))
}

pub async fn broadcast_message(
    State(websocket_service): State<Arc<WebSocketService>>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let message_type = payload["type"]
        .as_str()
        .unwrap_or("broadcast")
        .to_string();
    
    let data = payload["data"].clone();
    
    match websocket_service.broadcast_to_all(message_type, data).await {
        Ok(_) => Ok(Json(json!({
            "success": true,
            "message": "Broadcast sent successfully"
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "error": e.to_string()
            })),
        )),
    }
}

pub async fn send_message_to_user(
    State(websocket_service): State<Arc<WebSocketService>>,
    axum::extract::Path(user_id): axum::extract::Path<i32>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let message_type = payload["type"]
        .as_str()
        .unwrap_or("message")
        .to_string();
    
    let data = payload["data"].clone();
    
    match websocket_service.send_to_user(user_id, message_type, data).await {
        Ok(_) => Ok(Json(json!({
            "success": true,
            "message": format!("Message sent to user {} successfully", user_id)
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "error": e.to_string()
            })),
        )),
    }
}

pub async fn send_note_update(
    State(websocket_service): State<Arc<WebSocketService>>,
    axum::extract::Path((user_id, note_id)): axum::extract::Path<(i32, i32)>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let action = payload["action"]
        .as_str()
        .unwrap_or("updated");
    
    let data = payload["data"].clone();
    
    match websocket_service.send_note_update(user_id, note_id, action, data).await {
        Ok(_) => Ok(Json(json!({
            "success": true,
            "message": format!("Note {} notification sent to user {}", action, user_id)
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "error": e.to_string()
            })),
        )),
    }
}

pub async fn send_folder_update(
    State(websocket_service): State<Arc<WebSocketService>>,
    axum::extract::Path((user_id, folder_id)): axum::extract::Path<(i32, i32)>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let action = payload["action"]
        .as_str()
        .unwrap_or("updated");
    
    let data = payload["data"].clone();
    
    match websocket_service.send_folder_update(user_id, folder_id, action, data).await {
        Ok(_) => Ok(Json(json!({
            "success": true,
            "message": format!("Folder {} notification sent to user {}", action, user_id)
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "error": e.to_string()
            })),
        )),
    }
}

pub async fn send_sync_status(
    State(websocket_service): State<Arc<WebSocketService>>,
    axum::extract::Path(user_id): axum::extract::Path<i32>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let status = payload["status"]
        .as_str()
        .unwrap_or("unknown");
    
    let details = payload["details"].clone();
    
    match websocket_service.send_sync_status(user_id, status, details).await {
        Ok(_) => Ok(Json(json!({
            "success": true,
            "message": format!("Sync status sent to user {}", user_id)
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "error": e.to_string()
            })),
        )),
    }
}

pub async fn cleanup_connections(
    State(websocket_service): State<Arc<WebSocketService>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    websocket_service.cleanup_stale_connections().await;
    
    Ok(Json(json!({
        "success": true,
        "message": "Stale connections cleaned up"
    })))
}

pub async fn websocket_health_check(
    State(websocket_service): State<Arc<WebSocketService>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let stats = websocket_service.get_stats().await;
    let connected_users = websocket_service.get_connected_users().await;
    
    let health_status = if connected_users.is_empty() {
        "idle"
    } else {
        "active"
    };
    
    Ok(Json(json!({
        "success": true,
        "data": {
            "status": health_status,
            "stats": stats,
            "timestamp": chrono::Utc::now()
        }
    })))
}

// Middleware for WebSocket authentication (if needed for REST endpoints)
pub async fn websocket_auth_middleware(
    State(auth_service): State<Arc<AuthService>>,
    mut req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, (StatusCode, Json<Value>)> {
    // Extract token from header or query parameter
    let token = req
        .headers()
        .get("authorization")
        .and_then(|header| header.to_str().ok())
        .and_then(|header| header.strip_prefix("Bearer "))
        .or_else(|| {
            req.uri()
                .query()
                .and_then(|query| {
                    url::form_urlencoded::parse(query.as_bytes())
                        .find(|(key, _)| key == "token")
                        .map(|(_, value)| value.as_ref())
                })
        });

    let token = match token {
        Some(t) => t,
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "success": false,
                    "error": "Authentication token required"
                })),
            ));
        }
    };

    // Validate token
    let user = match auth_service.get_user_by_token(token).await {
        Ok(user) => user,
        Err(e) => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "success": false,
                    "error": format!("Authentication failed: {}", e)
                })),
            ));
        }
    };

    // Add user to request extensions
    req.extensions_mut().insert(user);

    Ok(next.run(req).await)
}