use deadpool_redis::{Config, Pool, Runtime};
use redis::{AsyncCommands, RedisResult};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{error, info};

#[derive(Debug)]
pub struct RedisManager {
    pool: Pool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    pub data: T,
    pub expires_at: i64,
}

impl RedisManager {
    /// Initialize Redis connection pool from environment
    pub async fn new(redis_url: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let cfg = Config::from_url(redis_url);
        let pool = cfg.create_pool(Some(Runtime::Tokio1))?;

        // Test the connection
        let mut conn = pool.get().await?;
        let _: String = redis::cmd("PING").query_async(&mut conn).await?;

        info!("Redis connection pool initialized successfully");

        Ok(Self { pool })
    }

    /// Get a Redis connection from the pool
    pub async fn get_connection(
        &self,
    ) -> Result<deadpool_redis::Connection, deadpool_redis::PoolError> {
        self.pool.get().await
    }

    /// Cache data with expiration (in seconds)
    pub async fn set_with_expiration<T>(
        &self,
        key: &str,
        value: &T,
        expiration_seconds: u64,
    ) -> RedisResult<()>
    where
        T: Serialize,
    {
        let mut conn = self.pool.get().await.map_err(|e| {
            error!("Failed to get Redis connection: {}", e);
            redis::RedisError::from((redis::ErrorKind::IoError, "Connection pool error"))
        })?;

        let serialized = serde_json::to_string(value).map_err(|e| {
            error!("Failed to serialize cache value: {}", e);
            redis::RedisError::from((redis::ErrorKind::TypeError, "Serialization error"))
        })?;

        conn.set_ex(key, serialized, expiration_seconds).await
    }

    /// Get cached data
    pub async fn get<T>(&self, key: &str) -> RedisResult<Option<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        let mut conn = self.pool.get().await.map_err(|e| {
            error!("Failed to get Redis connection: {}", e);
            redis::RedisError::from((redis::ErrorKind::IoError, "Connection pool error"))
        })?;

        let result: Option<String> = conn.get(key).await?;

        match result {
            Some(serialized) => {
                let deserialized: T = serde_json::from_str(&serialized).map_err(|e| {
                    error!("Failed to deserialize cached value: {}", e);
                    redis::RedisError::from((redis::ErrorKind::TypeError, "Deserialization error"))
                })?;
                Ok(Some(deserialized))
            }
            None => Ok(None),
        }
    }

    /// Delete cached data
    pub async fn delete(&self, key: &str) -> RedisResult<bool> {
        let mut conn = self.pool.get().await.map_err(|e| {
            error!("Failed to get Redis connection: {}", e);
            redis::RedisError::from((redis::ErrorKind::IoError, "Connection pool error"))
        })?;

        let result: i32 = conn.del(key).await?;
        Ok(result > 0)
    }

    /// Check if key exists
    pub async fn exists(&self, key: &str) -> RedisResult<bool> {
        let mut conn = self.pool.get().await.map_err(|e| {
            error!("Failed to get Redis connection: {}", e);
            redis::RedisError::from((redis::ErrorKind::IoError, "Connection pool error"))
        })?;

        conn.exists(key).await
    }

    /// Set expiration on existing key
    pub async fn expire(&self, key: &str, seconds: u64) -> RedisResult<bool> {
        let mut conn = self.pool.get().await.map_err(|e| {
            error!("Failed to get Redis connection: {}", e);
            redis::RedisError::from((redis::ErrorKind::IoError, "Connection pool error"))
        })?;

        conn.expire(key, seconds as i64).await
    }

    /// Increment counter with optional expiration
    pub async fn increment_with_expiry(&self, key: &str, expiry_seconds: u64) -> RedisResult<i64> {
        let mut conn = self.pool.get().await.map_err(|e| {
            error!("Failed to get Redis connection: {}", e);
            redis::RedisError::from((redis::ErrorKind::IoError, "Connection pool error"))
        })?;

        // Use Redis transaction to increment and set expiry atomically
        let result: i64 = redis::pipe()
            .incr(key, 1)
            .expire(key, expiry_seconds as i64)
            .query_async(&mut conn)
            .await?;

        Ok(result)
    }

    /// Flush all cached data (use with caution!)
    pub async fn flush_all(&self) -> RedisResult<()> {
        let mut conn = self.pool.get().await.map_err(|e| {
            error!("Failed to get Redis connection: {}", e);
            redis::RedisError::from((redis::ErrorKind::IoError, "Connection pool error"))
        })?;

        redis::cmd("FLUSHALL").query_async(&mut conn).await
    }
}

/// Utility functions for common caching patterns
impl RedisManager {
    /// Cache user session data
    pub async fn cache_user_session(
        &self,
        session_id: &str,
        user_id: &str,
        ttl_hours: u64,
    ) -> RedisResult<()> {
        let key = format!("session:{}", session_id);
        self.set_with_expiration(&key, &user_id, ttl_hours * 3600)
            .await
    }

    /// Get user ID from session
    pub async fn get_user_from_session(&self, session_id: &str) -> RedisResult<Option<String>> {
        let key = format!("session:{}", session_id);
        self.get(&key).await
    }

    /// Cache folder structure for a user
    pub async fn cache_folder_structure<T>(
        &self,
        user_id: &str,
        folders: &T,
        ttl_minutes: u64,
    ) -> RedisResult<()>
    where
        T: Serialize,
    {
        let key = format!("folders:{}", user_id);
        self.set_with_expiration(&key, folders, ttl_minutes * 60)
            .await
    }

    /// Get cached folder structure
    pub async fn get_cached_folder_structure<T>(&self, user_id: &str) -> RedisResult<Option<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        let key = format!("folders:{}", user_id);
        self.get(&key).await
    }

    /// Invalidate user's cached data
    pub async fn invalidate_user_cache(&self, user_id: &str) -> RedisResult<()> {
        let mut conn = self.pool.get().await.map_err(|e| {
            error!("Failed to get Redis connection: {}", e);
            redis::RedisError::from((redis::ErrorKind::IoError, "Connection pool error"))
        })?;

        // Delete all keys matching user pattern
        let pattern = format!("*:{}:*", user_id);
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(&pattern)
            .query_async(&mut conn)
            .await?;

        if !keys.is_empty() {
            conn.del(&keys).await?;
        }

        Ok(())
    }

    /// Rate limiting: Check if user has exceeded limit
    pub async fn check_rate_limit(
        &self,
        user_id: &str,
        endpoint: &str,
        limit: u64,
        window_seconds: u64,
    ) -> RedisResult<(bool, u64)> {
        let key = format!("rate_limit:{}:{}", user_id, endpoint);
        let current = self.increment_with_expiry(&key, window_seconds).await?;

        let allowed = current <= limit as i64;
        Ok((allowed, current as u64))
    }

    /// Health check for Redis connection
    pub async fn health_check(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.pool.get().await?;
        let _: String = redis::cmd("PING").query_async(&mut conn).await?;
        Ok(())
    }
}

/// Cache key generation utilities
pub mod cache_keys {
    /// Generate session cache key
    pub fn session(session_id: &str) -> String {
        format!("session:{}", session_id)
    }

    /// Generate user folder cache key  
    pub fn user_folders(user_id: &str) -> String {
        format!("folders:{}", user_id)
    }

    /// Generate recent notes cache key
    pub fn recent_notes(user_id: &str, limit: u32) -> String {
        format!("recent_notes:{}:{}", user_id, limit)
    }

    /// Generate search results cache key
    pub fn search_results(user_id: &str, query: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        query.hash(&mut hasher);
        let query_hash = hasher.finish();

        format!("search:{}:{:x}", user_id, query_hash)
    }

    /// Generate rate limit key
    pub fn rate_limit(user_id: &str, endpoint: &str) -> String {
        format!("rate_limit:{}:{}", user_id, endpoint)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Integration tests would require a running Redis instance
    // These are unit tests for cache key generation

    #[test]
    fn test_cache_key_generation() {
        assert_eq!(
            cache_keys::session("abc123"),
            "session:abc123"
        );

        assert_eq!(
            cache_keys::user_folders("user-456"),
            "folders:user-456"
        );

        assert_eq!(
            cache_keys::rate_limit("user-123", "/api/notes"),
            "rate_limit:user-123:/api/notes"
        );
    }

    #[test]
    fn test_search_cache_key_consistency() {
        let user_id = "user-123";
        let query = "test query";

        // Same query should generate same cache key
        let key1 = cache_keys::search_results(user_id, query);
        let key2 = cache_keys::search_results(user_id, query);
        assert_eq!(key1, key2);

        // Different queries should generate different cache keys
        let key3 = cache_keys::search_results(user_id, "different query");
        assert_ne!(key1, key3);
    }
}