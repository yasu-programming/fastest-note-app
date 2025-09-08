use crate::{
    repositories::{UserRepository, FolderRepository, NoteRepository},
    services::{AuthService, FolderService, NoteService, WebSocketService},
    middleware::rate_limit::RateLimiter,
};
use anyhow::Result;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub auth_service: Arc<AuthService>,
    pub folder_service: Arc<FolderService>,
    pub note_service: Arc<NoteService>,
    pub websocket_service: Arc<WebSocketService>,
    pub rate_limiter: Arc<RateLimiter>,
    pub pg_pool: sqlx::PgPool,
    pub redis_pool: deadpool_redis::Pool,
}

impl AppState {
    pub async fn new(config: AppConfig) -> Result<Self> {
        // Create database connection pool
        let pg_pool = create_database_pool(&config.database_url, config.database_max_connections).await?;
        
        // Create Redis connection pool
        let redis_pool = create_redis_pool(&config.redis_url, config.redis_max_connections)?;
        
        // Create repositories
        let user_repo = UserRepository::new(pg_pool.clone());
        let folder_repo = FolderRepository::new(pg_pool.clone());
        let note_repo = NoteRepository::new(pg_pool.clone());
        
        // Create services
        let auth_service = Arc::new(AuthService::new(
            user_repo,
            redis_pool.clone(),
            config.jwt_secret.clone(),
        ));
        
        let folder_service = Arc::new(FolderService::new(
            folder_repo,
            redis_pool.clone(),
        ));
        
        let websocket_service = Arc::new(WebSocketService::new());
        
        let note_service = Arc::new(NoteService::new(
            note_repo,
            redis_pool.clone(),
        ).with_websocket_service(websocket_service.clone()));
        
        let rate_limiter = Arc::new(RateLimiter::new(redis_pool.clone()));

        Ok(Self {
            auth_service,
            folder_service,
            note_service,
            websocket_service,
            rate_limiter,
            pg_pool,
            redis_pool,
        })
    }

    pub async fn health_check(&self) -> Result<HealthStatus> {
        let mut checks = std::collections::HashMap::new();

        // Database health check
        let db_start = std::time::Instant::now();
        match sqlx::query("SELECT 1").execute(&self.pg_pool).await {
            Ok(_) => {
                checks.insert("database".to_string(), ServiceHealth {
                    status: "healthy".to_string(),
                    response_time_ms: db_start.elapsed().as_millis() as u64,
                    last_checked: chrono::Utc::now(),
                    details: Some("Connection successful".to_string()),
                });
            }
            Err(e) => {
                checks.insert("database".to_string(), ServiceHealth {
                    status: "unhealthy".to_string(),
                    response_time_ms: db_start.elapsed().as_millis() as u64,
                    last_checked: chrono::Utc::now(),
                    details: Some(format!("Connection failed: {}", e)),
                });
            }
        }

        // Redis health check
        let redis_start = std::time::Instant::now();
        match self.redis_pool.get().await {
            Ok(mut conn) => {
                match redis::cmd("PING").query_async::<_, String>(&mut conn).await {
                    Ok(_) => {
                        checks.insert("redis".to_string(), ServiceHealth {
                            status: "healthy".to_string(),
                            response_time_ms: redis_start.elapsed().as_millis() as u64,
                            last_checked: chrono::Utc::now(),
                            details: Some("PING successful".to_string()),
                        });
                    }
                    Err(e) => {
                        checks.insert("redis".to_string(), ServiceHealth {
                            status: "unhealthy".to_string(),
                            response_time_ms: redis_start.elapsed().as_millis() as u64,
                            last_checked: chrono::Utc::now(),
                            details: Some(format!("PING failed: {}", e)),
                        });
                    }
                }
            }
            Err(e) => {
                checks.insert("redis".to_string(), ServiceHealth {
                    status: "unhealthy".to_string(),
                    response_time_ms: redis_start.elapsed().as_millis() as u64,
                    last_checked: chrono::Utc::now(),
                    details: Some(format!("Connection failed: {}", e)),
                });
            }
        }

        // WebSocket health check
        let ws_stats = self.websocket_service.get_stats().await;
        checks.insert("websocket".to_string(), ServiceHealth {
            status: "healthy".to_string(),
            response_time_ms: 0,
            last_checked: chrono::Utc::now(),
            details: Some(format!("Active connections: {}", ws_stats["total_connections"])),
        });

        // Determine overall status
        let overall_status = if checks.values().all(|check| check.status == "healthy") {
            "healthy"
        } else if checks.values().any(|check| check.status == "unhealthy") {
            "unhealthy"
        } else {
            "degraded"
        };

        Ok(HealthStatus {
            status: overall_status.to_string(),
            timestamp: chrono::Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            checks,
            system_info: get_system_info(),
        })
    }

    pub async fn get_metrics(&self) -> Result<AppMetrics> {
        // In a real implementation, these would be collected from actual monitoring systems
        let db_pool_status = self.pg_pool.acquire().await.is_ok();
        let redis_pool_status = self.redis_pool.get().await.is_ok();

        Ok(AppMetrics {
            timestamp: chrono::Utc::now(),
            uptime_seconds: get_uptime_seconds(),
            requests_total: 0, // Would be collected from middleware
            requests_per_second: 0.0,
            average_response_time_ms: 0.0,
            error_rate_percent: 0.0,
            active_websocket_connections: self.websocket_service.get_connected_users().await.len(),
            database_pool: PoolMetrics {
                active: 0, // Would need to implement pool introspection
                idle: 10,
                max: 20,
                healthy: db_pool_status,
            },
            redis_pool: PoolMetrics {
                active: 0,
                idle: 16,
                max: 16,
                healthy: redis_pool_status,
            },
            memory_usage_bytes: get_memory_usage(),
            cpu_usage_percent: get_cpu_usage(),
        })
    }

    pub async fn graceful_shutdown(&self) {
        tracing::info!("Starting graceful shutdown...");

        // Close database pool
        self.pg_pool.close().await;
        tracing::info!("Database pool closed");

        // Redis pool will be dropped automatically
        tracing::info!("Redis pool closed");

        // WebSocket cleanup
        self.websocket_service.cleanup_stale_connections().await;
        tracing::info!("WebSocket connections cleaned up");

        tracing::info!("Graceful shutdown completed");
    }
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub database_url: String,
    pub database_max_connections: u32,
    pub redis_url: String,
    pub redis_max_connections: usize,
    pub jwt_secret: String,
    pub server_host: String,
    pub server_port: u16,
    pub log_level: String,
    pub environment: Environment,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Environment {
    Development,
    Testing,
    Production,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgresql://localhost/fastest_note_app".to_string()),
            database_max_connections: std::env::var("DATABASE_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "20".to_string())
                .parse()
                .unwrap_or(20),
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            redis_max_connections: std::env::var("REDIS_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "16".to_string())
                .parse()
                .unwrap_or(16),
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "your-super-secret-jwt-key-change-in-production".to_string()),
            server_host: std::env::var("SERVER_HOST")
                .unwrap_or_else(|_| "0.0.0.0".to_string()),
            server_port: std::env::var("SERVER_PORT")
                .unwrap_or_else(|_| "8000".to_string())
                .parse()
                .unwrap_or(8000),
            log_level: std::env::var("LOG_LEVEL")
                .unwrap_or_else(|_| "info".to_string()),
            environment: match std::env::var("ENVIRONMENT").as_deref() {
                Ok("production") => Environment::Production,
                Ok("testing") => Environment::Testing,
                _ => Environment::Development,
            },
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub version: String,
    pub checks: std::collections::HashMap<String, ServiceHealth>,
    pub system_info: SystemInfo,
}

#[derive(Debug, serde::Serialize)]
pub struct ServiceHealth {
    pub status: String,
    pub response_time_ms: u64,
    pub last_checked: chrono::DateTime<chrono::Utc>,
    pub details: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct SystemInfo {
    pub hostname: String,
    pub platform: String,
    pub architecture: String,
    pub rust_version: String,
}

#[derive(Debug, serde::Serialize)]
pub struct AppMetrics {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub uptime_seconds: u64,
    pub requests_total: u64,
    pub requests_per_second: f64,
    pub average_response_time_ms: f64,
    pub error_rate_percent: f64,
    pub active_websocket_connections: usize,
    pub database_pool: PoolMetrics,
    pub redis_pool: PoolMetrics,
    pub memory_usage_bytes: u64,
    pub cpu_usage_percent: f64,
}

#[derive(Debug, serde::Serialize)]
pub struct PoolMetrics {
    pub active: u32,
    pub idle: u32,
    pub max: u32,
    pub healthy: bool,
}

// Helper functions
async fn create_database_pool(database_url: &str, max_connections: u32) -> Result<sqlx::PgPool> {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect(database_url)
        .await?;

    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}

fn create_redis_pool(redis_url: &str, max_connections: usize) -> Result<deadpool_redis::Pool> {
    let manager = deadpool_redis::Manager::new(redis_url)?;
    let pool = deadpool_redis::Pool::new(manager, max_connections)?;
    Ok(pool)
}

fn get_system_info() -> SystemInfo {
    SystemInfo {
        hostname: gethostname::gethostname().to_string_lossy().to_string(),
        platform: std::env::consts::OS.to_string(),
        architecture: std::env::consts::ARCH.to_string(),
        rust_version: env!("RUSTC_VERSION").to_string(),
    }
}

static START_TIME: std::sync::LazyLock<std::time::Instant> = std::sync::LazyLock::new(|| std::time::Instant::now());

fn get_uptime_seconds() -> u64 {
    START_TIME.elapsed().as_secs()
}

fn get_memory_usage() -> u64 {
    // In a real implementation, you would use a system monitoring library
    // For now, return 0 as placeholder
    0
}

fn get_cpu_usage() -> f64 {
    // In a real implementation, you would use a system monitoring library
    // For now, return 0.0 as placeholder
    0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_config_from_env() {
        std::env::set_var("DATABASE_URL", "postgresql://test/db");
        std::env::set_var("REDIS_URL", "redis://test:6379");
        std::env::set_var("JWT_SECRET", "test-secret");
        
        let config = AppConfig::default();
        
        assert_eq!(config.database_url, "postgresql://test/db");
        assert_eq!(config.redis_url, "redis://test:6379");
        assert_eq!(config.jwt_secret, "test-secret");
        
        // Cleanup
        std::env::remove_var("DATABASE_URL");
        std::env::remove_var("REDIS_URL");
        std::env::remove_var("JWT_SECRET");
    }

    #[test]
    fn test_environment_parsing() {
        std::env::set_var("ENVIRONMENT", "production");
        let config = AppConfig::default();
        assert_eq!(config.environment, Environment::Production);
        
        std::env::set_var("ENVIRONMENT", "testing");
        let config = AppConfig::default();
        assert_eq!(config.environment, Environment::Testing);
        
        std::env::remove_var("ENVIRONMENT");
        let config = AppConfig::default();
        assert_eq!(config.environment, Environment::Development);
    }
}