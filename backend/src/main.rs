use fastest_note_app_backend::{
    app_state::{AppConfig, AppState},
    router::create_app_router,
    shutdown::{run_server_with_graceful_shutdown, BackgroundTaskManager, spawn_health_check_task, spawn_rate_limit_cleanup_task},
};
use anyhow::Result;
use std::{net::SocketAddr, sync::Arc};
use tracing::{info, error, Level};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize configuration
    let config = AppConfig::default();
    
    // Initialize logging
    init_logging(&config)?;
    
    info!("Starting Fastest Note App Backend v{}", env!("CARGO_PKG_VERSION"));
    info!("Environment: {:?}", config.environment);
    info!("Database URL: {}", mask_password(&config.database_url));
    info!("Redis URL: {}", mask_password(&config.redis_url));
    
    // Initialize application state
    info!("Initializing application state...");
    let app_state = Arc::new(AppState::new(config.clone()).await?);
    info!("Application state initialized successfully");
    
    // Perform initial health check
    info!("Performing initial health check...");
    match app_state.health_check().await {
        Ok(health) => {
            if health.status == "healthy" {
                info!("Initial health check passed");
            } else {
                error!("Initial health check failed with status: {}", health.status);
                return Err(anyhow::anyhow!("Initial health check failed"));
            }
        }
        Err(e) => {
            error!("Initial health check error: {}", e);
            return Err(e.into());
        }
    }
    
    // Create the application router
    info!("Creating application router...");
    let app = create_app_router(
        app_state.auth_service.clone(),
        app_state.folder_service.clone(),
        app_state.note_service.clone(),
        app_state.websocket_service.clone(),
        app_state.rate_limiter.clone(),
    );
    
    // Start background tasks
    info!("Starting background tasks...");
    let mut task_manager = BackgroundTaskManager::new();
    
    // Health check task
    let health_check_app_state = app_state.clone();
    task_manager.spawn_task("health_check", async move {
        let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);
        spawn_health_check_task(health_check_app_state, shutdown_rx).await;
    });
    
    // Rate limit cleanup task
    let rate_limit_redis_pool = app_state.redis_pool.clone();
    task_manager.spawn_task("rate_limit_cleanup", async move {
        let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);
        spawn_rate_limit_cleanup_task(rate_limit_redis_pool, shutdown_rx).await;
    });
    
    info!("Background tasks started");
    
    // Prepare server address
    let bind_addr = SocketAddr::from(([0, 0, 0, 0], config.server_port));
    
    // Print startup information
    print_startup_banner(&config, &bind_addr);
    
    // Start the server with graceful shutdown
    info!("Starting HTTP server...");
    let server_result = run_server_with_graceful_shutdown(app, app_state.clone(), bind_addr).await;
    
    // Shutdown background tasks
    info!("Shutting down background tasks...");
    task_manager.shutdown_all().await;
    
    // Handle server result
    match server_result {
        Ok(_) => {
            info!("Server shut down gracefully");
            Ok(())
        }
        Err(e) => {
            error!("Server error: {}", e);
            Err(e.into())
        }
    }
}

fn init_logging(config: &AppConfig) -> Result<()> {
    // Parse log level
    let log_level = match config.log_level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };
    
    // Create subscriber based on environment
    let subscriber = tracing_subscriber::registry();
    
    match config.environment {
        fastest_note_app_backend::app_state::Environment::Production => {
            // JSON logging for production
            let json_layer = tracing_subscriber::fmt::layer()
                .json()
                .with_target(true)
                .with_thread_ids(true)
                .with_level(true)
                .with_file(false)
                .with_line_number(false);
            
            subscriber
                .with(tracing_subscriber::filter::LevelFilter::from_level(log_level))
                .with(json_layer)
                .init();
        }
        fastest_note_app_backend::app_state::Environment::Development | 
        fastest_note_app_backend::app_state::Environment::Testing => {
            // Pretty logging for development
            let fmt_layer = tracing_subscriber::fmt::layer()
                .pretty()
                .with_target(false)
                .with_thread_ids(false)
                .with_file(true)
                .with_line_number(true);
            
            subscriber
                .with(tracing_subscriber::filter::LevelFilter::from_level(log_level))
                .with(fmt_layer)
                .init();
        }
    }
    
    info!("Logging initialized with level: {}", log_level);
    Ok(())
}

fn mask_password(url: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url) {
        if parsed.password().is_some() {
            let mut masked = parsed.clone();
            masked.set_password(Some("***")).unwrap();
            return masked.to_string();
        }
    }
    url.to_string()
}

fn print_startup_banner(config: &AppConfig, bind_addr: &SocketAddr) {
    println!("
    ╔══════════════════════════════════════════════════════════════╗
    ║                    Fastest Note App Backend                  ║
    ║                      Version {}                      ║
    ╠══════════════════════════════════════════════════════════════╣
    ║  🚀 Server starting on: {}                     ║
    ║  🌍 Environment: {:?}                                ║
    ║  📊 Health Check: http://{}{}              ║
    ║  📈 Metrics: http://{}{}                   ║
    ║  🔌 WebSocket: ws://{}{}                    ║
    ║  📝 API Documentation: {}            ║
    ╚══════════════════════════════════════════════════════════════╝
    ",
        env!("CARGO_PKG_VERSION"),
        bind_addr,
        config.environment,
        bind_addr, "/health",
        bind_addr, "/metrics", 
        bind_addr, "/ws",
        "Coming soon!"
    );
    
    info!("🎉 Fastest Note App Backend is ready to serve requests!");
    info!("📚 API Endpoints:");
    info!("   • Authentication: /auth/*");
    info!("   • Notes: /notes/*");
    info!("   • Folders: /folders/*");
    info!("   • WebSocket: /ws");
    info!("   • Health Check: /health");
    info!("   • Metrics: /metrics");
    info!("   • System Info: /system-info");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_password() {
        let url_with_password = "postgresql://user:password@localhost:5432/db";
        let masked = mask_password(url_with_password);
        assert!(masked.contains("***"));
        assert!(!masked.contains("password"));
    }

    #[test]
    fn test_mask_password_no_password() {
        let url_without_password = "postgresql://localhost:5432/db";
        let masked = mask_password(url_without_password);
        assert_eq!(masked, url_without_password);
    }

    #[test]
    fn test_mask_password_invalid_url() {
        let invalid_url = "not-a-url";
        let masked = mask_password(invalid_url);
        assert_eq!(masked, invalid_url);
    }

    #[tokio::test]
    async fn test_app_config_creation() {
        let config = AppConfig::default();
        assert!(!config.database_url.is_empty());
        assert!(!config.redis_url.is_empty());
        assert!(!config.jwt_secret.is_empty());
        assert!(config.server_port > 0);
    }
}
