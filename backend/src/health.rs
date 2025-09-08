use crate::app_state::{AppState, HealthStatus, ServiceHealth, SystemInfo, AppMetrics, PoolMetrics};
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
};
use serde_json::{json, Value};
use std::sync::Arc;

pub async fn basic_health_check() -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "service": "fastest-note-app",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

pub async fn detailed_health_check(
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match app_state.health_check().await {
        Ok(health_status) => {
            let status_code = match health_status.status.as_str() {
                "healthy" => StatusCode::OK,
                "degraded" => StatusCode::OK, // Still OK but with warnings
                "unhealthy" => StatusCode::SERVICE_UNAVAILABLE,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };

            Ok((status_code, Json(serde_json::to_value(health_status).unwrap())).into())
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "status": "error",
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "error": e.to_string()
            })),
        )),
    }
}

pub async fn liveness_probe() -> Json<Value> {
    // Simple liveness probe - just checks if the service is running
    Json(json!({
        "status": "alive",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

pub async fn readiness_probe(
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Readiness probe - checks if service is ready to handle requests
    
    // Quick database check
    let db_ready = match sqlx::query("SELECT 1").execute(&app_state.pg_pool).await {
        Ok(_) => true,
        Err(_) => false,
    };

    // Quick Redis check
    let redis_ready = match app_state.redis_pool.get().await {
        Ok(mut conn) => {
            match redis::cmd("PING").query_async::<_, String>(&mut conn).await {
                Ok(_) => true,
                Err(_) => false,
            }
        }
        Err(_) => false,
    };

    let ready = db_ready && redis_ready;

    if ready {
        Ok(Json(json!({
            "status": "ready",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "checks": {
                "database": db_ready,
                "redis": redis_ready
            }
        })))
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "not_ready",
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "checks": {
                    "database": db_ready,
                    "redis": redis_ready
                }
            })),
        ))
    }
}

pub async fn startup_probe() -> Json<Value> {
    // Startup probe - checks if service has started successfully
    Json(json!({
        "status": "started",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "uptime_seconds": crate::app_state::get_uptime_seconds()
    }))
}

pub async fn metrics_endpoint(
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match app_state.get_metrics().await {
        Ok(metrics) => Ok(Json(serde_json::to_value(metrics).unwrap())),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "Failed to collect metrics",
                "details": e.to_string(),
                "timestamp": chrono::Utc::now().to_rfc3339()
            })),
        )),
    }
}

pub async fn prometheus_metrics_endpoint(
    State(app_state): State<Arc<AppState>>,
) -> Result<String, (StatusCode, Json<Value>)> {
    match app_state.get_metrics().await {
        Ok(metrics) => {
            let prometheus_format = format!(
                r#"# HELP app_uptime_seconds Application uptime in seconds
# TYPE app_uptime_seconds counter
app_uptime_seconds {}

# HELP app_requests_total Total number of requests
# TYPE app_requests_total counter
app_requests_total {}

# HELP app_requests_per_second Current requests per second
# TYPE app_requests_per_second gauge
app_requests_per_second {}

# HELP app_response_time_ms Average response time in milliseconds
# TYPE app_response_time_ms gauge
app_response_time_ms {}

# HELP app_error_rate Error rate as percentage
# TYPE app_error_rate gauge
app_error_rate {}

# HELP app_websocket_connections Active WebSocket connections
# TYPE app_websocket_connections gauge
app_websocket_connections {}

# HELP app_database_pool_active Active database connections
# TYPE app_database_pool_active gauge
app_database_pool_active {}

# HELP app_database_pool_idle Idle database connections
# TYPE app_database_pool_idle gauge
app_database_pool_idle {}

# HELP app_database_pool_max Maximum database connections
# TYPE app_database_pool_max gauge
app_database_pool_max {}

# HELP app_redis_pool_active Active Redis connections
# TYPE app_redis_pool_active gauge
app_redis_pool_active {}

# HELP app_redis_pool_idle Idle Redis connections
# TYPE app_redis_pool_idle gauge
app_redis_pool_idle {}

# HELP app_redis_pool_max Maximum Redis connections
# TYPE app_redis_pool_max gauge
app_redis_pool_max {}

# HELP app_memory_usage_bytes Memory usage in bytes
# TYPE app_memory_usage_bytes gauge
app_memory_usage_bytes {}

# HELP app_cpu_usage_percent CPU usage percentage
# TYPE app_cpu_usage_percent gauge
app_cpu_usage_percent {}
"#,
                metrics.uptime_seconds,
                metrics.requests_total,
                metrics.requests_per_second,
                metrics.average_response_time_ms,
                metrics.error_rate_percent,
                metrics.active_websocket_connections,
                metrics.database_pool.active,
                metrics.database_pool.idle,
                metrics.database_pool.max,
                metrics.redis_pool.active,
                metrics.redis_pool.idle,
                metrics.redis_pool.max,
                metrics.memory_usage_bytes,
                metrics.cpu_usage_percent,
            );

            Ok(prometheus_format)
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "Failed to collect metrics",
                "details": e.to_string(),
                "timestamp": chrono::Utc::now().to_rfc3339()
            })),
        )),
    }
}

pub async fn database_health_check(
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let start = std::time::Instant::now();
    
    match sqlx::query("SELECT version(), current_database(), current_user, now()").fetch_one(&app_state.pg_pool).await {
        Ok(row) => {
            let version: String = row.get(0);
            let database: String = row.get(1);
            let user: String = row.get(2);
            let timestamp: chrono::DateTime<chrono::Utc> = row.get(3);
            
            Ok(Json(json!({
                "status": "healthy",
                "response_time_ms": start.elapsed().as_millis(),
                "database_info": {
                    "version": version,
                    "database": database,
                    "user": user,
                    "server_time": timestamp.to_rfc3339()
                },
                "checked_at": chrono::Utc::now().to_rfc3339()
            })))
        }
        Err(e) => Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "unhealthy",
                "error": e.to_string(),
                "response_time_ms": start.elapsed().as_millis(),
                "checked_at": chrono::Utc::now().to_rfc3339()
            })),
        )),
    }
}

pub async fn redis_health_check(
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let start = std::time::Instant::now();
    
    match app_state.redis_pool.get().await {
        Ok(mut conn) => {
            match redis::cmd("INFO").arg("server").query_async::<_, String>(&mut conn).await {
                Ok(info) => {
                    // Parse Redis version from INFO response
                    let version = info.lines()
                        .find(|line| line.starts_with("redis_version:"))
                        .map(|line| line.split(':').nth(1).unwrap_or("unknown"))
                        .unwrap_or("unknown");

                    Ok(Json(json!({
                        "status": "healthy",
                        "response_time_ms": start.elapsed().as_millis(),
                        "redis_info": {
                            "version": version,
                            "ping": "PONG"
                        },
                        "checked_at": chrono::Utc::now().to_rfc3339()
                    })))
                }
                Err(e) => Err((
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(json!({
                        "status": "unhealthy",
                        "error": format!("Redis command failed: {}", e),
                        "response_time_ms": start.elapsed().as_millis(),
                        "checked_at": chrono::Utc::now().to_rfc3339()
                    })),
                )),
            }
        }
        Err(e) => Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "unhealthy",
                "error": format!("Redis connection failed: {}", e),
                "response_time_ms": start.elapsed().as_millis(),
                "checked_at": chrono::Utc::now().to_rfc3339()
            })),
        )),
    }
}

pub async fn websocket_health_check(
    State(app_state): State<Arc<AppState>>,
) -> Json<Value> {
    let stats = app_state.websocket_service.get_stats().await;
    let connected_users = app_state.websocket_service.get_connected_users().await;

    Json(json!({
        "status": "healthy",
        "stats": stats,
        "connected_users": connected_users.len(),
        "checked_at": chrono::Utc::now().to_rfc3339()
    }))
}

pub async fn system_info_endpoint() -> Json<Value> {
    let system_info = crate::app_state::get_system_info();
    
    Json(json!({
        "system": system_info,
        "process": {
            "pid": std::process::id(),
            "uptime_seconds": crate::app_state::get_uptime_seconds(),
            "memory_usage_bytes": crate::app_state::get_memory_usage(),
            "cpu_usage_percent": crate::app_state::get_cpu_usage()
        },
        "runtime": {
            "rust_version": env!("RUSTC_VERSION"),
            "cargo_version": env!("CARGO_PKG_VERSION"),
            "build_timestamp": env!("BUILD_TIMESTAMP").unwrap_or("unknown"),
            "git_commit": env!("GIT_COMMIT").unwrap_or("unknown")
        },
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_basic_health_check() {
        let response = basic_health_check().await;
        let body = response.0;
        
        assert!(body["status"].as_str().unwrap() == "healthy");
        assert!(body["service"].as_str().unwrap() == "fastest-note-app");
    }

    #[tokio::test]
    async fn test_liveness_probe() {
        let response = liveness_probe().await;
        let body = response.0;
        
        assert!(body["status"].as_str().unwrap() == "alive");
    }

    #[tokio::test]
    async fn test_startup_probe() {
        let response = startup_probe().await;
        let body = response.0;
        
        assert!(body["status"].as_str().unwrap() == "started");
        assert!(body["uptime_seconds"].is_number());
    }
}