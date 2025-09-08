use anyhow::{Context, Result};
use deadpool_redis::{Pool, PoolError, Connection};
use redis::{AsyncCommands, RedisError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Cache configuration settings
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub default_ttl: Duration,
    pub max_connections: usize,
    pub connection_timeout: Duration,
    pub enable_compression: bool,
    pub key_prefix: String,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            default_ttl: Duration::from_secs(3600), // 1 hour
            max_connections: 10,
            connection_timeout: Duration::from_secs(5),
            enable_compression: false,
            key_prefix: "fastest_note_app".to_string(),
        }
    }
}

/// Cache statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub sets: u64,
    pub deletes: u64,
    pub errors: u64,
    pub hit_rate: f64,
    pub last_reset: u64,
}

impl CacheStats {
    fn new() -> Self {
        Self {
            hits: 0,
            misses: 0,
            sets: 0,
            deletes: 0,
            errors: 0,
            hit_rate: 0.0,
            last_reset: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    fn calculate_hit_rate(&mut self) {
        let total_requests = self.hits + self.misses;
        self.hit_rate = if total_requests > 0 {
            (self.hits as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };
    }
}

/// Cached data wrapper with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedData<T> {
    pub data: T,
    pub cached_at: u64,
    pub expires_at: u64,
    pub version: u32,
}

impl<T> CachedData<T> {
    pub fn new(data: T, ttl: Duration) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Self {
            data,
            cached_at: now,
            expires_at: now + ttl.as_secs(),
            version: 1,
        }
    }

    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now > self.expires_at
    }
}

/// High-performance Redis cache service
#[derive(Clone)]
pub struct RedisCacheService {
    pool: Pool,
    config: CacheConfig,
    stats: std::sync::Arc<tokio::sync::Mutex<CacheStats>>,
}

impl RedisCacheService {
    /// Create a new Redis cache service
    pub async fn new(pool: Pool, config: CacheConfig) -> Result<Self> {
        let service = Self {
            pool,
            config,
            stats: std::sync::Arc::new(tokio::sync::Mutex::new(CacheStats::new())),
        };

        // Test connection
        service.ping().await.context("Failed to connect to Redis")?;
        info!("Redis cache service initialized successfully");

        Ok(service)
    }

    /// Test Redis connection
    pub async fn ping(&self) -> Result<()> {
        let mut conn = self.get_connection().await?;
        let _: String = conn.ping().await.context("Redis ping failed")?;
        Ok(())
    }

    /// Get Redis connection from pool
    async fn get_connection(&self) -> Result<Connection> {
        self.pool
            .get()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get Redis connection: {}", e))
    }

    /// Build cache key with prefix and namespace
    fn build_key(&self, namespace: &str, key: &str) -> String {
        format!("{}:{}:{}", self.config.key_prefix, namespace, key)
    }

    /// Serialize data with optional compression
    fn serialize<T: Serialize>(&self, data: &T) -> Result<Vec<u8>> {
        let serialized = bincode::serialize(data)
            .context("Failed to serialize data")?;

        if self.config.enable_compression && serialized.len() > 1024 {
            // Use compression for data larger than 1KB
            use flate2::write::GzEncoder;
            use flate2::Compression;
            use std::io::Write;

            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&serialized)
                .context("Failed to compress data")?;
            encoder.finish()
                .context("Failed to finalize compression")
        } else {
            Ok(serialized)
        }
    }

    /// Deserialize data with optional decompression
    fn deserialize<T: for<'de> Deserialize<'de>>(&self, data: &[u8]) -> Result<T> {
        let decompressed = if self.config.enable_compression {
            // Try to decompress first, fall back to raw data
            match self.decompress(data) {
                Ok(decompressed) => decompressed,
                Err(_) => data.to_vec(), // Assume raw data
            }
        } else {
            data.to_vec()
        };

        bincode::deserialize(&decompressed)
            .context("Failed to deserialize data")
    }

    /// Decompress data
    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        use flate2::read::GzDecoder;
        use std::io::Read;

        let mut decoder = GzDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)
            .context("Failed to decompress data")?;
        Ok(decompressed)
    }

    /// Update cache statistics
    async fn update_stats<F>(&self, update_fn: F) 
    where 
        F: FnOnce(&mut CacheStats),
    {
        let mut stats = self.stats.lock().await;
        update_fn(&mut stats);
        stats.calculate_hit_rate();
    }

    /// Get data from cache
    pub async fn get<T>(&self, namespace: &str, key: &str) -> Result<Option<T>>
    where
        T: for<'de> Deserialize<'de> + Clone,
    {
        let cache_key = self.build_key(namespace, key);
        
        match self.get_connection().await {
            Ok(mut conn) => {
                match conn.get::<_, Option<Vec<u8>>>(&cache_key).await {
                    Ok(Some(data)) => {
                        match self.deserialize::<CachedData<T>>(&data) {
                            Ok(cached) => {
                                if cached.is_expired() {
                                    // Remove expired data
                                    let _ = conn.del::<_, ()>(&cache_key).await;
                                    self.update_stats(|stats| stats.misses += 1).await;
                                    debug!("Cache expired for key: {}", cache_key);
                                    Ok(None)
                                } else {
                                    self.update_stats(|stats| stats.hits += 1).await;
                                    debug!("Cache hit for key: {}", cache_key);
                                    Ok(Some(cached.data))
                                }
                            }
                            Err(e) => {
                                error!("Failed to deserialize cached data for key {}: {}", cache_key, e);
                                let _ = conn.del::<_, ()>(&cache_key).await;
                                self.update_stats(|stats| {
                                    stats.misses += 1;
                                    stats.errors += 1;
                                }).await;
                                Ok(None)
                            }
                        }
                    }
                    Ok(None) => {
                        self.update_stats(|stats| stats.misses += 1).await;
                        debug!("Cache miss for key: {}", cache_key);
                        Ok(None)
                    }
                    Err(e) => {
                        error!("Redis get error for key {}: {}", cache_key, e);
                        self.update_stats(|stats| stats.errors += 1).await;
                        Ok(None)
                    }
                }
            }
            Err(e) => {
                error!("Failed to get Redis connection: {}", e);
                self.update_stats(|stats| stats.errors += 1).await;
                Ok(None)
            }
        }
    }

    /// Set data in cache with TTL
    pub async fn set<T>(&self, namespace: &str, key: &str, data: &T, ttl: Option<Duration>) -> Result<()>
    where
        T: Serialize + Clone,
    {
        let cache_key = self.build_key(namespace, key);
        let ttl = ttl.unwrap_or(self.config.default_ttl);
        let cached_data = CachedData::new(data.clone(), ttl);

        match self.serialize(&cached_data) {
            Ok(serialized) => {
                match self.get_connection().await {
                    Ok(mut conn) => {
                        match conn.setex::<_, _, ()>(&cache_key, ttl.as_secs() as u64, serialized).await {
                            Ok(_) => {
                                self.update_stats(|stats| stats.sets += 1).await;
                                debug!("Cached data for key: {} with TTL: {}s", cache_key, ttl.as_secs());
                                Ok(())
                            }
                            Err(e) => {
                                error!("Redis set error for key {}: {}", cache_key, e);
                                self.update_stats(|stats| stats.errors += 1).await;
                                Err(anyhow::anyhow!("Failed to set cache: {}", e))
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to get Redis connection: {}", e);
                        self.update_stats(|stats| stats.errors += 1).await;
                        Err(e)
                    }
                }
            }
            Err(e) => {
                error!("Failed to serialize data for key {}: {}", cache_key, e);
                self.update_stats(|stats| stats.errors += 1).await;
                Err(e)
            }
        }
    }

    /// Delete data from cache
    pub async fn delete(&self, namespace: &str, key: &str) -> Result<bool> {
        let cache_key = self.build_key(namespace, key);

        match self.get_connection().await {
            Ok(mut conn) => {
                match conn.del::<_, u32>(&cache_key).await {
                    Ok(count) => {
                        let deleted = count > 0;
                        self.update_stats(|stats| stats.deletes += 1).await;
                        if deleted {
                            debug!("Deleted cache key: {}", cache_key);
                        }
                        Ok(deleted)
                    }
                    Err(e) => {
                        error!("Redis delete error for key {}: {}", cache_key, e);
                        self.update_stats(|stats| stats.errors += 1).await;
                        Err(anyhow::anyhow!("Failed to delete cache: {}", e))
                    }
                }
            }
            Err(e) => {
                error!("Failed to get Redis connection: {}", e);
                self.update_stats(|stats| stats.errors += 1).await;
                Err(e)
            }
        }
    }

    /// Delete multiple keys by pattern
    pub async fn delete_pattern(&self, namespace: &str, pattern: &str) -> Result<u32> {
        let search_pattern = self.build_key(namespace, pattern);
        
        match self.get_connection().await {
            Ok(mut conn) => {
                // Get all keys matching pattern
                let keys: Vec<String> = conn.keys(&search_pattern).await
                    .context("Failed to get keys by pattern")?;

                if keys.is_empty() {
                    return Ok(0);
                }

                // Delete all matching keys
                let deleted: u32 = conn.del(&keys).await
                    .context("Failed to delete keys")?;

                self.update_stats(|stats| stats.deletes += deleted as u64).await;
                debug!("Deleted {} keys matching pattern: {}", deleted, search_pattern);
                Ok(deleted)
            }
            Err(e) => {
                error!("Failed to get Redis connection: {}", e);
                self.update_stats(|stats| stats.errors += 1).await;
                Err(e)
            }
        }
    }

    /// Check if key exists in cache
    pub async fn exists(&self, namespace: &str, key: &str) -> Result<bool> {
        let cache_key = self.build_key(namespace, key);

        match self.get_connection().await {
            Ok(mut conn) => {
                let exists: bool = conn.exists(&cache_key).await
                    .context("Failed to check key existence")?;
                Ok(exists)
            }
            Err(e) => {
                error!("Failed to get Redis connection: {}", e);
                Err(e)
            }
        }
    }

    /// Increment counter
    pub async fn increment(&self, namespace: &str, key: &str, delta: i64) -> Result<i64> {
        let cache_key = self.build_key(namespace, key);

        match self.get_connection().await {
            Ok(mut conn) => {
                let value: i64 = conn.incr(&cache_key, delta).await
                    .context("Failed to increment counter")?;
                
                // Set TTL if this is a new key
                let _ = conn.expire(&cache_key, self.config.default_ttl.as_secs() as usize).await;
                
                Ok(value)
            }
            Err(e) => {
                error!("Failed to get Redis connection: {}", e);
                Err(e)
            }
        }
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> CacheStats {
        self.stats.lock().await.clone()
    }

    /// Reset cache statistics
    pub async fn reset_stats(&self) {
        let mut stats = self.stats.lock().await;
        *stats = CacheStats::new();
    }

    /// Get Redis info
    pub async fn get_redis_info(&self) -> Result<HashMap<String, String>> {
        let mut conn = self.get_connection().await?;
        let info: String = conn.info().await.context("Failed to get Redis info")?;
        
        let mut result = HashMap::new();
        for line in info.lines() {
            if let Some((key, value)) = line.split_once(':') {
                result.insert(key.to_string(), value.to_string());
            }
        }
        
        Ok(result)
    }

    /// Flush all cache data (use with caution!)
    pub async fn flush_all(&self) -> Result<()> {
        let mut conn = self.get_connection().await?;
        conn.flushdb().await.context("Failed to flush Redis database")?;
        
        // Reset stats
        self.reset_stats().await;
        warn!("Redis cache flushed completely");
        Ok(())
    }
}

/// Cache key builders for different entity types
pub mod keys {
    use uuid::Uuid;

    pub fn user_key(user_id: &Uuid) -> String {
        format!("user:{}", user_id)
    }

    pub fn note_key(note_id: &Uuid) -> String {
        format!("note:{}", note_id)
    }

    pub fn folder_key(folder_id: &Uuid) -> String {
        format!("folder:{}", folder_id)
    }

    pub fn user_notes_key(user_id: &Uuid, folder_id: Option<&Uuid>) -> String {
        match folder_id {
            Some(fid) => format!("user_notes:{}:{}", user_id, fid),
            None => format!("user_notes:{}", user_id),
        }
    }

    pub fn user_folders_key(user_id: &Uuid) -> String {
        format!("user_folders:{}", user_id)
    }

    pub fn search_key(user_id: &Uuid, query: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        query.hash(&mut hasher);
        let query_hash = hasher.finish();
        
        format!("search:{}:{:x}", user_id, query_hash)
    }

    pub fn session_key(session_id: &str) -> String {
        format!("session:{}", session_id)
    }

    pub fn rate_limit_key(user_id: &Uuid, endpoint: &str) -> String {
        format!("rate_limit:{}:{}", user_id, endpoint)
    }
}

/// High-level cache operations for specific entities
impl RedisCacheService {
    /// Cache user data
    pub async fn cache_user<T>(&self, user_id: &Uuid, data: &T, ttl: Option<Duration>) -> Result<()>
    where
        T: Serialize + Clone,
    {
        self.set("users", &keys::user_key(user_id), data, ttl).await
    }

    /// Get cached user data
    pub async fn get_user<T>(&self, user_id: &Uuid) -> Result<Option<T>>
    where
        T: for<'de> Deserialize<'de> + Clone,
    {
        self.get("users", &keys::user_key(user_id)).await
    }

    /// Cache note data
    pub async fn cache_note<T>(&self, note_id: &Uuid, data: &T, ttl: Option<Duration>) -> Result<()>
    where
        T: Serialize + Clone,
    {
        self.set("notes", &keys::note_key(note_id), data, ttl).await
    }

    /// Get cached note data
    pub async fn get_note<T>(&self, note_id: &Uuid) -> Result<Option<T>>
    where
        T: for<'de> Deserialize<'de> + Clone,
    {
        self.get("notes", &keys::note_key(note_id)).await
    }

    /// Invalidate user-related caches
    pub async fn invalidate_user_caches(&self, user_id: &Uuid) -> Result<()> {
        let patterns = [
            format!("user:{}", user_id),
            format!("user_notes:{}:*", user_id),
            format!("user_folders:{}", user_id),
            format!("search:{}:*", user_id),
        ];

        for pattern in &patterns {
            let _ = self.delete_pattern("", pattern).await; // Ignore individual failures
        }

        Ok(())
    }

    /// Cache search results with shorter TTL
    pub async fn cache_search_results<T>(&self, user_id: &Uuid, query: &str, results: &T) -> Result<()>
    where
        T: Serialize + Clone,
    {
        let ttl = Duration::from_secs(300); // 5 minutes for search results
        self.set("search", &keys::search_key(user_id, query), results, Some(ttl)).await
    }

    /// Get cached search results
    pub async fn get_search_results<T>(&self, user_id: &Uuid, query: &str) -> Result<Option<T>>
    where
        T: for<'de> Deserialize<'de> + Clone,
    {
        self.get("search", &keys::search_key(user_id, query)).await
    }

    /// Rate limiting support
    pub async fn check_rate_limit(&self, user_id: &Uuid, endpoint: &str, limit: u32, window: Duration) -> Result<bool> {
        let key = keys::rate_limit_key(user_id, endpoint);
        let current = self.increment("rate_limit", &key, 1).await?;
        
        if current == 1 {
            // Set expiry only for the first increment
            let cache_key = self.build_key("rate_limit", &key);
            let mut conn = self.get_connection().await?;
            let _ = conn.expire(&cache_key, window.as_secs() as usize).await;
        }

        Ok(current as u32 <= limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestData {
        id: u32,
        name: String,
        active: bool,
    }

    // Mock tests would go here
    // In a real implementation, you'd use a test Redis instance
}