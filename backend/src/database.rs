use deadpool_postgres::{Config, ManagerConfig, Pool, PoolConfig, RecyclingMethod, Runtime};
use sqlx::migrate::MigrateDatabase;
use sqlx::{PgPool, Postgres, Row};
use std::time::Duration;
use tokio_postgres::{Client, NoTls};
use tracing::{error, info, warn};

#[derive(Debug)]
pub struct DatabaseManager {
    pub pool: Pool,
    pub sqlx_pool: PgPool,
}

impl DatabaseManager {
    /// Initialize database connection pools
    pub async fn new(
        database_url: &str,
        max_connections: u32,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Create database if it doesn't exist
        if !sqlx::Postgres::database_exists(database_url).await? {
            info!("Database does not exist, creating...");
            sqlx::Postgres::create_database(database_url).await?;
            info!("Database created successfully");
        }

        // Initialize SQLx pool for migrations and complex queries
        let sqlx_pool = PgPool::connect(database_url).await?;

        // Run migrations
        sqlx::migrate!("./migrations").run(&sqlx_pool).await?;
        info!("Database migrations completed successfully");

        // Initialize deadpool for general connection pooling
        let mut cfg = Config::new();

        // Parse database URL to extract connection parameters
        let url = url::Url::parse(database_url)?;
        
        cfg.user = url.username().to_string().into();
        cfg.password = url.password().map(|p| p.to_string());
        cfg.host = url.host_str().map(|h| h.to_string());
        cfg.port = url.port();
        cfg.dbname = url.path().trim_start_matches('/').to_string().into();

        // Configure connection pool settings
        cfg.manager = Some(ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        });

        cfg.pool = Some(PoolConfig {
            max_size: max_connections as usize,
            timeouts: deadpool_postgres::Timeouts {
                wait: Some(Duration::from_secs(30)),
                create: Some(Duration::from_secs(30)),
                recycle: Some(Duration::from_secs(30)),
            },
        });

        let pool = cfg.create_pool(Some(Runtime::Tokio1), NoTls)?;

        // Test the connection
        let client = pool.get().await?;
        let _rows = client.query("SELECT 1", &[]).await?;

        info!(
            "PostgreSQL connection pool initialized successfully with {} max connections",
            max_connections
        );

        Ok(Self { pool, sqlx_pool })
    }

    /// Get a client connection from the pool
    pub async fn get_client(&self) -> Result<deadpool_postgres::Client, deadpool_postgres::PoolError> {
        self.pool.get().await
    }

    /// Get SQLx pool for complex queries and transactions
    pub fn get_sqlx_pool(&self) -> &PgPool {
        &self.sqlx_pool
    }

    /// Health check for database connections
    pub async fn health_check(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Test deadpool connection
        let client = self.pool.get().await?;
        let _row = client.query_one("SELECT 1 as health", &[]).await?;

        // Test SQLx connection
        let _row = sqlx::query("SELECT 1 as health")
            .fetch_one(&self.sqlx_pool)
            .await?;

        info!("Database health check passed");
        Ok(())
    }

    /// Get database statistics
    pub async fn get_stats(&self) -> Result<DatabaseStats, Box<dyn std::error::Error + Send + Sync>> {
        let client = self.pool.get().await?;

        // Query for database statistics
        let stats_query = "
            SELECT 
                (SELECT COUNT(*) FROM users) as user_count,
                (SELECT COUNT(*) FROM folders) as folder_count,
                (SELECT COUNT(*) FROM notes) as note_count,
                (SELECT pg_database_size(current_database())) as db_size_bytes,
                (SELECT COUNT(*) FROM pg_stat_activity WHERE state = 'active') as active_connections
        ";

        let row = client.query_one(stats_query, &[]).await?;

        Ok(DatabaseStats {
            user_count: row.get::<_, i64>(0) as u64,
            folder_count: row.get::<_, i64>(1) as u64,
            note_count: row.get::<_, i64>(2) as u64,
            db_size_bytes: row.get::<_, i64>(3) as u64,
            active_connections: row.get::<_, i64>(4) as u32,
            pool_size: self.pool.status().size as u32,
            available_connections: self.pool.status().available as u32,
        })
    }

    /// Close all connections gracefully
    pub async fn close(&self) {
        info!("Closing database connections...");
        
        // Close SQLx pool
        self.sqlx_pool.close().await;
        
        // Deadpool closes connections automatically when dropped
        info!("Database connections closed successfully");
    }

    /// Execute a database transaction with retry logic
    pub async fn execute_transaction<T, F, Fut>(
        &self,
        operation: F,
    ) -> Result<T, Box<dyn std::error::Error + Send + Sync>>
    where
        F: Fn(sqlx::Transaction<'_, Postgres>) -> Fut,
        Fut: std::future::Future<Output = Result<T, sqlx::Error>>,
    {
        const MAX_RETRIES: u32 = 3;
        let mut retries = 0;

        loop {
            let mut tx = self.sqlx_pool.begin().await?;

            match operation(tx).await {
                Ok(result) => {
                    return Ok(result);
                }
                Err(e) => {
                    retries += 1;
                    
                    if retries >= MAX_RETRIES {
                        error!("Transaction failed after {} retries: {}", MAX_RETRIES, e);
                        return Err(Box::new(e));
                    }

                    warn!("Transaction failed (attempt {}), retrying: {}", retries, e);
                    
                    // Wait before retrying (exponential backoff)
                    tokio::time::sleep(Duration::from_millis(100 * retries as u64)).await;
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct DatabaseStats {
    pub user_count: u64,
    pub folder_count: u64,  
    pub note_count: u64,
    pub db_size_bytes: u64,
    pub active_connections: u32,
    pub pool_size: u32,
    pub available_connections: u32,
}

/// Database query utilities
pub mod queries {
    use sqlx::PgPool;
    use uuid::Uuid;

    /// Check if user exists by ID
    pub async fn user_exists(pool: &PgPool, user_id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM users WHERE id = $1)",
            user_id
        )
        .fetch_one(pool)
        .await?;

        Ok(result.unwrap_or(false))
    }

    /// Get user by email
    pub async fn get_user_by_email(pool: &PgPool, email: &str) -> Result<Option<Uuid>, sqlx::Error> {
        let result = sqlx::query_scalar!(
            "SELECT id FROM users WHERE email = $1",
            email
        )
        .fetch_optional(pool)
        .await?;

        Ok(result)
    }

    /// Get folder count for user
    pub async fn get_folder_count(pool: &PgPool, user_id: Uuid) -> Result<i64, sqlx::Error> {
        let result = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM folders WHERE user_id = $1",
            user_id
        )
        .fetch_one(pool)
        .await?;

        Ok(result.unwrap_or(0))
    }

    /// Get note count for user
    pub async fn get_note_count(pool: &PgPool, user_id: Uuid) -> Result<i64, sqlx::Error> {
        let result = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM notes WHERE user_id = $1",
            user_id
        )
        .fetch_one(pool)
        .await?;

        Ok(result.unwrap_or(0))
    }

    /// Get notes in folder with pagination
    pub async fn get_notes_in_folder(
        pool: &PgPool,
        user_id: Uuid,
        folder_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<NoteListItem>, sqlx::Error> {
        let results = sqlx::query_as!(
            NoteListItem,
            r#"
            SELECT 
                id,
                title,
                content_size,
                updated_at
            FROM notes 
            WHERE user_id = $1 AND folder_id IS NOT DISTINCT FROM $2
            ORDER BY updated_at DESC
            LIMIT $3 OFFSET $4
            "#,
            user_id,
            folder_id,
            limit,
            offset
        )
        .fetch_all(pool)
        .await?;

        Ok(results)
    }

    /// Validate folder hierarchy depth
    pub async fn validate_folder_depth(
        pool: &PgPool,
        parent_folder_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query_scalar!(
            "SELECT level FROM folders WHERE id = $1 AND user_id = $2",
            parent_folder_id,
            user_id
        )
        .fetch_optional(pool)
        .await?;

        match result {
            Some(Some(level)) => Ok(level < 9), // Allow one more level (max 10)
            _ => Ok(false), // Parent folder doesn't exist or belongs to different user
        }
    }

    #[derive(Debug)]
    pub struct NoteListItem {
        pub id: Uuid,
        pub title: String,
        pub content_size: Option<i32>,
        pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    }
}

/// Database connection utilities
pub mod connection {
    use super::*;

    /// Create a new database manager from environment variables
    pub async fn from_env() -> Result<DatabaseManager, Box<dyn std::error::Error + Send + Sync>> {
        let database_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set");
        
        let max_connections: u32 = std::env::var("DATABASE_MAX_CONNECTIONS")
            .unwrap_or_else(|_| "10".to_string())
            .parse()
            .expect("DATABASE_MAX_CONNECTIONS must be a valid number");

        DatabaseManager::new(&database_url, max_connections).await
    }

    /// Wait for database to become available with timeout
    pub async fn wait_for_database(
        database_url: &str,
        timeout_seconds: u64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(timeout_seconds);

        loop {
            match PgPool::connect(database_url).await {
                Ok(pool) => {
                    pool.close().await;
                    info!("Database is available");
                    return Ok(());
                }
                Err(e) => {
                    if start.elapsed() > timeout {
                        error!("Database connection timeout after {} seconds", timeout_seconds);
                        return Err(Box::new(e));
                    }
                    
                    warn!("Waiting for database to become available: {}", e);
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Integration tests would require a running PostgreSQL instance
    // These are unit tests for utility functions

    #[test]
    fn test_database_stats_creation() {
        let stats = DatabaseStats {
            user_count: 100,
            folder_count: 500,
            note_count: 2000,
            db_size_bytes: 1024 * 1024 * 100, // 100MB
            active_connections: 5,
            pool_size: 10,
            available_connections: 5,
        };

        assert_eq!(stats.user_count, 100);
        assert_eq!(stats.folder_count, 500);
        assert_eq!(stats.note_count, 2000);
    }

    #[test]
    fn test_connection_url_parsing() {
        let database_url = "postgresql://user:pass@localhost:5432/dbname";
        let parsed = url::Url::parse(database_url).expect("Valid URL");
        
        assert_eq!(parsed.username(), "user");
        assert_eq!(parsed.password(), Some("pass"));
        assert_eq!(parsed.host_str(), Some("localhost"));
        assert_eq!(parsed.port(), Some(5432));
        assert_eq!(parsed.path(), "/dbname");
    }
}