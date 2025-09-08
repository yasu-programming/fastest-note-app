use fastest_note_backend::{DatabaseManager, RedisManager};
use std::env;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "fastest_note_backend=debug,tower_http=debug,axum=debug".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Fast Note-Taking Backend Server");

    // Load environment variables
    dotenvy::dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    
    let redis_url = env::var("REDIS_URL")
        .expect("REDIS_URL must be set");

    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("PORT")
        .unwrap_or_else(|_| "3001".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid port number");

    // Initialize database connection
    info!("Initializing database connection...");
    let db_manager = DatabaseManager::new(&database_url, 10).await?;
    
    // Initialize Redis connection
    info!("Initializing Redis connection...");  
    let redis_manager = RedisManager::new(&redis_url).await?;

    // Perform health checks
    info!("Performing health checks...");
    if let Err(e) = db_manager.health_check().await {
        error!("Database health check failed: {}", e);
        std::process::exit(1);
    }

    if let Err(e) = redis_manager.health_check().await {
        error!("Redis health check failed: {}", e);
        std::process::exit(1);
    }

    info!("All health checks passed");

    // TODO: Initialize Axum app with routes (will be implemented in later tasks)
    info!("Server would start on {}:{}", host, port);
    info!("Database and Redis connections established successfully");
    info!("Ready for API implementation...");

    // For now, just keep the server running
    tokio::signal::ctrl_c().await?;
    info!("Shutting down gracefully...");

    db_manager.close().await;
    info!("Server shut down complete");

    Ok(())
}
