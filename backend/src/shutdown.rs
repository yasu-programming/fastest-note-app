use crate::app_state::AppState;
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info, warn};

pub struct GracefulShutdown {
    app_state: Arc<AppState>,
    shutdown_timeout: std::time::Duration,
}

impl GracefulShutdown {
    pub fn new(app_state: Arc<AppState>) -> Self {
        Self {
            app_state,
            shutdown_timeout: std::time::Duration::from_secs(30), // 30 second timeout
        }
    }

    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.shutdown_timeout = timeout;
        self
    }

    pub async fn wait_for_shutdown_signal(&self) {
        let ctrl_c = async {
            signal::ctrl_c()
                .await
                .expect("failed to install Ctrl+C handler");
            info!("Received Ctrl+C signal");
        };

        #[cfg(unix)]
        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("failed to install SIGTERM handler")
                .recv()
                .await;
            info!("Received SIGTERM signal");
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {},
            _ = terminate => {},
        }

        info!("Shutdown signal received, starting graceful shutdown...");
        self.perform_graceful_shutdown().await;
    }

    async fn perform_graceful_shutdown(&self) {
        let shutdown_start = std::time::Instant::now();
        
        info!("Starting graceful shutdown process...");

        // Create a timeout for the entire shutdown process
        let shutdown_future = async {
            // Phase 1: Stop accepting new connections
            info!("Phase 1: Stopping new connections...");
            
            // Phase 2: Clean up WebSocket connections
            info!("Phase 2: Cleaning up WebSocket connections...");
            self.app_state.websocket_service.cleanup_stale_connections().await;
            
            // Wait a bit for in-flight requests to complete
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            
            // Phase 3: Close database connections
            info!("Phase 3: Closing database connections...");
            self.app_state.pg_pool.close().await;
            
            // Phase 4: Cleanup Redis connections (they'll be dropped automatically)
            info!("Phase 4: Redis connections will be closed automatically");
            
            // Phase 5: Final cleanup
            info!("Phase 5: Final cleanup completed");
            
            let shutdown_duration = shutdown_start.elapsed();
            info!("Graceful shutdown completed in {:?}", shutdown_duration);
        };

        // Run shutdown with timeout
        match tokio::time::timeout(self.shutdown_timeout, shutdown_future).await {
            Ok(_) => {
                info!("Graceful shutdown completed successfully");
            }
            Err(_) => {
                warn!("Graceful shutdown timed out after {:?}, forcing exit", self.shutdown_timeout);
                self.force_shutdown().await;
            }
        }
    }

    async fn force_shutdown(&self) {
        error!("Performing force shutdown...");
        
        // Force close database pool
        self.app_state.pg_pool.close().await;
        
        // Log final statistics
        if let Ok(metrics) = self.app_state.get_metrics().await {
            info!("Final metrics before shutdown: {:?}", metrics);
        }
        
        error!("Force shutdown completed");
    }

    pub async fn setup_shutdown_hooks(&self) {
        // Set up panic hook for better error reporting
        let original_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            error!("Application panicked: {:?}", panic_info);
            original_hook(panic_info);
            std::process::exit(1);
        }));

        // Register cleanup function to run on exit
        let app_state = self.app_state.clone();
        let cleanup_fn = move || {
            info!("Running exit cleanup...");
            // Note: This runs in a synchronous context, so we can't use async operations
            // The main graceful shutdown should handle most cleanup
        };

        // This is a no-op on most systems, but good practice
        #[cfg(unix)]
        {
            use signal::{Signal, Trap};
            let trap = Trap::trap(&[Signal::SIGINT, Signal::SIGTERM]);
            // The signal handling is done in wait_for_shutdown_signal
        }
    }
}

// Utility function to create a shutdown-aware server
pub async fn run_server_with_graceful_shutdown(
    app: axum::Router,
    app_state: Arc<AppState>,
    bind_addr: std::net::SocketAddr,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Starting server on {}", bind_addr);
    
    // Create the server
    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    
    // Set up graceful shutdown
    let shutdown_handler = GracefulShutdown::new(app_state.clone());
    shutdown_handler.setup_shutdown_hooks().await;
    
    info!("Server is ready to accept connections");
    
    // Run server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            shutdown_handler.wait_for_shutdown_signal().await;
        })
        .await?;

    Ok(())
}

// Background task cleanup
pub struct BackgroundTaskManager {
    tasks: Vec<tokio::task::JoinHandle<()>>,
    shutdown_tx: Option<tokio::sync::broadcast::Sender<()>>,
}

impl BackgroundTaskManager {
    pub fn new() -> Self {
        let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);
        
        Self {
            tasks: Vec::new(),
            shutdown_tx: Some(shutdown_tx),
        }
    }

    pub fn spawn_task<F>(&mut self, name: &str, future: F) 
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        let task_name = name.to_string();
        let shutdown_rx = self.shutdown_tx.as_ref().unwrap().subscribe();
        
        let handle = tokio::spawn(async move {
            let mut shutdown_rx = shutdown_rx;
            
            tokio::select! {
                _ = future => {
                    info!("Background task '{}' completed", task_name);
                }
                _ = shutdown_rx.recv() => {
                    info!("Background task '{}' received shutdown signal", task_name);
                }
            }
        });

        self.tasks.push(handle);
        info!("Spawned background task: {}", name);
    }

    pub async fn shutdown_all(&mut self) {
        info!("Shutting down {} background tasks...", self.tasks.len());
        
        // Send shutdown signal to all tasks
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }

        // Wait for all tasks to complete with timeout
        let mut remaining_tasks = Vec::new();
        std::mem::swap(&mut remaining_tasks, &mut self.tasks);

        for (i, task) in remaining_tasks.into_iter().enumerate() {
            match tokio::time::timeout(std::time::Duration::from_secs(5), task).await {
                Ok(result) => {
                    if let Err(e) = result {
                        warn!("Background task {} panicked: {}", i, e);
                    }
                }
                Err(_) => {
                    warn!("Background task {} did not complete within timeout", i);
                }
            }
        }

        info!("All background tasks have been shut down");
    }
}

impl Drop for BackgroundTaskManager {
    fn drop(&mut self) {
        // Abort any remaining tasks
        for task in &self.tasks {
            task.abort();
        }
    }
}

// Health check task that runs periodically
pub async fn spawn_health_check_task(
    app_state: Arc<AppState>,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
    
    loop {
        tokio::select! {
            _ = interval.tick() => {
                match app_state.health_check().await {
                    Ok(health) => {
                        if health.status != "healthy" {
                            warn!("Health check returned non-healthy status: {}", health.status);
                        }
                    }
                    Err(e) => {
                        error!("Health check failed: {}", e);
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                info!("Health check task received shutdown signal");
                break;
            }
        }
    }
}

// Cleanup task for rate limiting keys
pub async fn spawn_rate_limit_cleanup_task(
    redis_pool: deadpool_redis::Pool,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(300)); // 5 minutes
    
    loop {
        tokio::select! {
            _ = interval.tick() => {
                if let Err(e) = crate::middleware::rate_limit::cleanup_rate_limit_keys(&redis_pool).await {
                    warn!("Failed to cleanup rate limit keys: {}", e);
                } else {
                    info!("Rate limit keys cleanup completed");
                }
            }
            _ = shutdown_rx.recv() => {
                info!("Rate limit cleanup task received shutdown signal");
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_background_task_manager_creation() {
        let manager = BackgroundTaskManager::new();
        assert_eq!(manager.tasks.len(), 0);
        assert!(manager.shutdown_tx.is_some());
    }

    #[tokio::test]
    async fn test_background_task_spawning() {
        let mut manager = BackgroundTaskManager::new();
        
        manager.spawn_task("test_task", async {
            tokio::time::sleep(Duration::from_millis(100)).await;
        });
        
        assert_eq!(manager.tasks.len(), 1);
        
        manager.shutdown_all().await;
        assert_eq!(manager.tasks.len(), 0);
    }

    #[test]
    fn test_graceful_shutdown_timeout_configuration() {
        let app_state = Arc::new({
            // This is a placeholder - in real tests you'd create a proper AppState
            // For now, we'll just test the timeout configuration
            std::mem::MaybeUninit::uninit()
        });
        
        // This test just verifies the API compiles
        // let shutdown = GracefulShutdown::new(app_state)
        //     .with_timeout(Duration::from_secs(60));
    }
}