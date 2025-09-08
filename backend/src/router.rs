use crate::{
    handlers::{auth, folder, note, websocket},
    middleware::{
        auth::{auth_middleware, optional_auth_middleware},
        cors::dev_cors_middleware,
        rate_limit::{global_rate_limit_middleware, auth_rate_limit_middleware, user_rate_limit_middleware, api_rate_limit_middleware, RateLimiter},
        logging::{request_logging_middleware, security_logging_middleware, performance_monitoring_middleware},
    },
    services::{AuthService, FolderService, NoteService, WebSocketService},
};
use axum::{
    middleware,
    routing::{delete, get, patch, post, put},
    Router,
};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::timeout::TimeoutLayer;

pub fn create_app_router(
    auth_service: Arc<AuthService>,
    folder_service: Arc<FolderService>,
    note_service: Arc<NoteService>,
    websocket_service: Arc<WebSocketService>,
    rate_limiter: Arc<RateLimiter>,
) -> Router {
    // Create the main router with global middleware
    let app = Router::new()
        .merge(health_routes())
        .merge(auth_routes(auth_service.clone()))
        .merge(protected_routes(
            auth_service.clone(),
            folder_service,
            note_service,
            websocket_service.clone(),
        ))
        .merge(websocket_routes(websocket_service, auth_service.clone()))
        .layer(
            ServiceBuilder::new()
                // Timeout layer - 30 seconds for all requests
                .layer(TimeoutLayer::new(std::time::Duration::from_secs(30)))
                // CORS layer (development mode)
                .layer(middleware::from_fn(dev_cors_middleware))
                // Security logging
                .layer(middleware::from_fn(security_logging_middleware))
                // Performance monitoring
                .layer(middleware::from_fn(performance_monitoring_middleware))
                // Request logging
                .layer(middleware::from_fn(request_logging_middleware))
                // Global rate limiting
                .layer(middleware::from_fn_with_state(
                    rate_limiter,
                    global_rate_limit_middleware,
                ))
        );

    app
}

fn health_routes() -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/health/detailed", get(detailed_health_check))
        .route("/metrics", get(metrics_endpoint))
}

fn auth_routes(auth_service: Arc<AuthService>) -> Router {
    Router::new()
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/auth/refresh", post(auth::refresh_token))
        .route("/auth/logout", post(auth::logout))
        .layer(
            ServiceBuilder::new()
                .layer(middleware::from_fn_with_state(
                    auth_service.clone(),
                    auth_rate_limit_middleware,
                ))
        )
        .with_state(auth_service)
}

fn protected_routes(
    auth_service: Arc<AuthService>,
    folder_service: Arc<FolderService>,
    note_service: Arc<NoteService>,
    websocket_service: Arc<WebSocketService>,
) -> Router {
    let user_routes = Router::new()
        .route("/auth/me", get(auth::get_me))
        .route("/auth/change-password", post(auth::change_password))
        .route("/auth/sessions", get(auth::get_sessions))
        .route("/auth/sessions/revoke-all", post(auth::revoke_all_sessions))
        .route("/auth/revoke-token", post(auth::revoke_token));

    let folder_routes = Router::new()
        .route("/folders", get(folder::get_user_folders))
        .route("/folders", post(folder::create_folder))
        .route("/folders/tree", get(folder::get_folder_tree))
        .route("/folders/search", get(folder::search_folders))
        .route("/folders/:id", get(folder::get_folder))
        .route("/folders/:id", put(folder::update_folder))
        .route("/folders/:id", delete(folder::delete_folder))
        .route("/folders/:id/move", patch(folder::move_folder))
        .route("/folders/:id/stats", get(folder::get_folder_stats))
        .route("/folders/:id/breadcrumbs", get(folder::get_folder_breadcrumbs))
        .route("/folders/:id/duplicate", post(folder::duplicate_folder))
        .route("/folders/:id/validate", get(folder::validate_folder_path));

    let note_routes = Router::new()
        .route("/notes", get(note::get_user_notes))
        .route("/notes", post(note::create_note))
        .route("/notes/recent", get(note::get_recent_notes))
        .route("/notes/search", get(note::search_notes))
        .route("/notes/count", get(note::get_note_count))
        .route("/notes/statistics", get(note::get_note_statistics))
        .route("/notes/analytics", get(note::get_note_analytics))
        .route("/notes/move", patch(note::move_notes))
        .route("/notes/batch-delete", delete(note::batch_delete_notes))
        .route("/notes/:id", get(note::get_note))
        .route("/notes/:id", put(note::update_note))
        .route("/notes/:id", delete(note::delete_note))
        .route("/notes/:id/duplicate", post(note::duplicate_note));

    Router::new()
        .merge(user_routes.with_state(auth_service.clone()))
        .merge(folder_routes.with_state(folder_service))
        .merge(note_routes.with_state(note_service))
        .layer(
            ServiceBuilder::new()
                // API-specific rate limiting
                .layer(middleware::from_fn_with_state(
                    RateLimiter::new(
                        // This should come from your app state, but for now we'll create a placeholder
                        deadpool_redis::Pool::new(
                            deadpool_redis::Manager::new("redis://localhost:6379").unwrap(),
                            16
                        ).unwrap()
                    ),
                    api_rate_limit_middleware,
                ))
                // User-specific rate limiting
                .layer(middleware::from_fn_with_state(
                    RateLimiter::new(
                        deadpool_redis::Pool::new(
                            deadpool_redis::Manager::new("redis://localhost:6379").unwrap(),
                            16
                        ).unwrap()
                    ),
                    user_rate_limit_middleware,
                ))
                // Authentication required
                .layer(middleware::from_fn_with_state(
                    auth_service,
                    auth_middleware,
                ))
        )
}

fn websocket_routes(
    websocket_service: Arc<WebSocketService>,
    auth_service: Arc<AuthService>,
) -> Router {
    let ws_routes = Router::new()
        .route("/ws", get(websocket::websocket_handler))
        .with_state((websocket_service.clone(), auth_service.clone()));

    let ws_api_routes = Router::new()
        .route("/ws/stats", get(websocket::get_websocket_stats))
        .route("/ws/connected-users", get(websocket::get_connected_users))
        .route("/ws/users/:id/connection", get(websocket::check_user_connection))
        .route("/ws/broadcast", post(websocket::broadcast_message))
        .route("/ws/users/:id/message", post(websocket::send_message_to_user))
        .route("/ws/users/:user_id/notes/:note_id/update", post(websocket::send_note_update))
        .route("/ws/users/:user_id/folders/:folder_id/update", post(websocket::send_folder_update))
        .route("/ws/users/:id/sync-status", post(websocket::send_sync_status))
        .route("/ws/cleanup", post(websocket::cleanup_connections))
        .route("/ws/health", get(websocket::websocket_health_check))
        .with_state((websocket_service, auth_service));

    Router::new()
        .merge(ws_routes)
        .merge(ws_api_routes)
}

// Health check handlers
async fn health_check() -> axum::response::Json<serde_json::Value> {
    axum::response::Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "version": env!("CARGO_PKG_VERSION")
    }))
}

async fn detailed_health_check() -> axum::response::Json<serde_json::Value> {
    // In a real implementation, you would check:
    // - Database connectivity
    // - Redis connectivity
    // - External service health
    // - System resources (memory, disk, etc.)
    
    let uptime = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    axum::response::Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "version": env!("CARGO_PKG_VERSION"),
        "uptime_seconds": uptime,
        "checks": {
            "database": {
                "status": "healthy",
                "response_time_ms": 2,
                "last_checked": chrono::Utc::now().to_rfc3339()
            },
            "redis": {
                "status": "healthy", 
                "response_time_ms": 1,
                "last_checked": chrono::Utc::now().to_rfc3339()
            },
            "websocket": {
                "status": "healthy",
                "active_connections": 0,
                "last_checked": chrono::Utc::now().to_rfc3339()
            }
        },
        "system": {
            "memory_usage": "N/A",
            "cpu_usage": "N/A",
            "disk_usage": "N/A"
        }
    }))
}

async fn metrics_endpoint() -> axum::response::Json<serde_json::Value> {
    // In a real implementation, you would collect actual metrics
    axum::response::Json(serde_json::json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "metrics": {
            "requests_total": 0,
            "requests_per_second": 0.0,
            "average_response_time_ms": 0.0,
            "error_rate": 0.0,
            "active_connections": 0,
            "database_pool": {
                "active": 0,
                "idle": 10,
                "max": 10
            },
            "redis_pool": {
                "active": 0,
                "idle": 16,
                "max": 16
            }
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        database::create_connection_pools,
        redis::create_redis_pool,
        repositories::{UserRepository, FolderRepository, NoteRepository},
        services::{AuthService, FolderService, NoteService, WebSocketService},
        middleware::rate_limit::RateLimiter,
    };
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_health_check_endpoint() {
        // Create minimal app for testing
        let app = Router::new()
            .route("/health", get(health_check));

        let request = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test] 
    async fn test_cors_headers() {
        let app = Router::new()
            .route("/health", get(health_check))
            .layer(middleware::from_fn(dev_cors_middleware));

        let request = Request::builder()
            .uri("/health")
            .header("Origin", "http://localhost:3000")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        
        assert!(response.headers().contains_key("access-control-allow-origin"));
    }
}