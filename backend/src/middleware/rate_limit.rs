use axum::{
    extract::{Request, State, ConnectInfo},
    http::{StatusCode, HeaderMap},
    middleware::Next,
    response::{IntoResponse, Json, Response},
};
use redis::AsyncCommands;
use serde_json::json;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::time::Instant;

#[derive(Clone)]
pub struct RateLimiter {
    redis_pool: deadpool_redis::Pool,
}

impl RateLimiter {
    pub fn new(redis_pool: deadpool_redis::Pool) -> Self {
        Self { redis_pool }
    }
}

pub struct RateLimitConfig {
    pub requests_per_window: u32,
    pub window_seconds: u32,
    pub burst_requests: Option<u32>, // Allow burst above normal limit
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_window: 100, // 100 requests per minute
            window_seconds: 60,
            burst_requests: Some(20), // Allow 20 extra requests for burst
        }
    }
}

// Global rate limiting middleware
pub async fn global_rate_limit_middleware(
    State(rate_limiter): State<Arc<RateLimiter>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    mut req: Request,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    let config = RateLimitConfig {
        requests_per_window: 1000, // 1000 requests per minute globally per IP
        window_seconds: 60,
        burst_requests: Some(100),
    };

    let client_ip = addr.ip().to_string();
    let key = format!("rate_limit:global:{}", client_ip);

    match check_rate_limit(&rate_limiter.redis_pool, &key, &config).await {
        Ok(rate_limit_info) => {
            // Add rate limit headers
            let mut response = next.run(req).await;
            let headers = response.headers_mut();
            
            headers.insert("X-RateLimit-Limit", rate_limit_info.limit.to_string().parse().unwrap());
            headers.insert("X-RateLimit-Remaining", rate_limit_info.remaining.to_string().parse().unwrap());
            headers.insert("X-RateLimit-Reset", rate_limit_info.reset_time.to_string().parse().unwrap());
            
            Ok(response)
        }
        Err(rate_limit_error) => {
            Ok((
                StatusCode::TOO_MANY_REQUESTS,
                Json(json!({
                    "success": false,
                    "error": "Rate limit exceeded",
                    "details": {
                        "limit": rate_limit_error.limit,
                        "window_seconds": rate_limit_error.window_seconds,
                        "retry_after": rate_limit_error.retry_after_seconds
                    }
                })),
            )
                .into_response())
        }
    }
}

// Authentication endpoint rate limiting (stricter)
pub async fn auth_rate_limit_middleware(
    State(rate_limiter): State<Arc<RateLimiter>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    let config = RateLimitConfig {
        requests_per_window: 10, // 10 auth requests per minute
        window_seconds: 60,
        burst_requests: None, // No burst for auth
    };

    let client_ip = addr.ip().to_string();
    let key = format!("rate_limit:auth:{}", client_ip);

    match check_rate_limit(&rate_limiter.redis_pool, &key, &config).await {
        Ok(_) => Ok(next.run(req).await),
        Err(rate_limit_error) => {
            Ok((
                StatusCode::TOO_MANY_REQUESTS,
                Json(json!({
                    "success": false,
                    "error": "Authentication rate limit exceeded",
                    "details": {
                        "limit": rate_limit_error.limit,
                        "window_seconds": rate_limit_error.window_seconds,
                        "retry_after": rate_limit_error.retry_after_seconds
                    }
                })),
            )
                .into_response())
        }
    }
}

// Per-user rate limiting middleware
pub async fn user_rate_limit_middleware(
    State(rate_limiter): State<Arc<RateLimiter>>,
    mut req: Request,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    // Get user from request extensions (added by auth middleware)
    let user_id = req
        .extensions()
        .get::<crate::models::User>()
        .map(|user| user.id);

    if let Some(user_id) = user_id {
        let config = RateLimitConfig {
            requests_per_window: 500, // 500 requests per minute per user
            window_seconds: 60,
            burst_requests: Some(50),
        };

        let key = format!("rate_limit:user:{}", user_id);

        match check_rate_limit(&rate_limiter.redis_pool, &key, &config).await {
            Ok(rate_limit_info) => {
                let mut response = next.run(req).await;
                let headers = response.headers_mut();
                
                headers.insert("X-RateLimit-Limit", rate_limit_info.limit.to_string().parse().unwrap());
                headers.insert("X-RateLimit-Remaining", rate_limit_info.remaining.to_string().parse().unwrap());
                headers.insert("X-RateLimit-Reset", rate_limit_info.reset_time.to_string().parse().unwrap());
                
                Ok(response)
            }
            Err(rate_limit_error) => {
                Ok((
                    StatusCode::TOO_MANY_REQUESTS,
                    Json(json!({
                        "success": false,
                        "error": "User rate limit exceeded",
                        "details": {
                            "limit": rate_limit_error.limit,
                            "window_seconds": rate_limit_error.window_seconds,
                            "retry_after": rate_limit_error.retry_after_seconds
                        }
                    })),
                )
                    .into_response())
            }
        }
    } else {
        // No user authenticated, continue without user-specific rate limiting
        Ok(next.run(req).await)
    }
}

// API endpoint rate limiting (for expensive operations)
pub async fn api_rate_limit_middleware(
    State(rate_limiter): State<Arc<RateLimiter>>,
    req: Request,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    let user_id = req
        .extensions()
        .get::<crate::models::User>()
        .map(|user| user.id);

    if let Some(user_id) = user_id {
        // Different limits for different endpoints
        let path = req.uri().path();
        let (limit, window, endpoint_type) = match path {
            p if p.contains("/search") => (20, 60, "search"), // 20 searches per minute
            p if p.contains("/notes") && req.method() == "POST" => (60, 60, "create"), // 60 creates per minute
            p if p.contains("/folders") && req.method() == "POST" => (30, 60, "folder_create"), // 30 folder creates per minute
            _ => (200, 60, "api"), // 200 other API calls per minute
        };

        let config = RateLimitConfig {
            requests_per_window: limit,
            window_seconds: window,
            burst_requests: Some(limit / 5), // 20% burst allowance
        };

        let key = format!("rate_limit:{}:{}:{}", endpoint_type, user_id, path);

        match check_rate_limit(&rate_limiter.redis_pool, &key, &config).await {
            Ok(_) => Ok(next.run(req).await),
            Err(rate_limit_error) => {
                Ok((
                    StatusCode::TOO_MANY_REQUESTS,
                    Json(json!({
                        "success": false,
                        "error": format!("{} rate limit exceeded", endpoint_type),
                        "details": {
                            "limit": rate_limit_error.limit,
                            "window_seconds": rate_limit_error.window_seconds,
                            "retry_after": rate_limit_error.retry_after_seconds,
                            "endpoint_type": endpoint_type
                        }
                    })),
                )
                    .into_response())
            }
        }
    } else {
        Ok(next.run(req).await)
    }
}

#[derive(Debug)]
struct RateLimitInfo {
    limit: u32,
    remaining: u32,
    reset_time: u64,
}

#[derive(Debug)]
struct RateLimitError {
    limit: u32,
    window_seconds: u32,
    retry_after_seconds: u32,
}

async fn check_rate_limit(
    redis_pool: &deadpool_redis::Pool,
    key: &str,
    config: &RateLimitConfig,
) -> Result<RateLimitInfo, RateLimitError> {
    let mut conn = redis_pool.get().await.map_err(|_| RateLimitError {
        limit: config.requests_per_window,
        window_seconds: config.window_seconds,
        retry_after_seconds: config.window_seconds,
    })?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let window_start = now - (now % config.window_seconds as u64);
    let window_key = format!("{}:{}", key, window_start);

    // Get current count
    let current_count: u32 = conn.get(&window_key).await.unwrap_or(0);
    
    // Calculate effective limit (including burst if applicable)
    let effective_limit = config.requests_per_window + config.burst_requests.unwrap_or(0);

    if current_count >= effective_limit {
        let retry_after = config.window_seconds - (now % config.window_seconds as u64) as u32;
        return Err(RateLimitError {
            limit: config.requests_per_window,
            window_seconds: config.window_seconds,
            retry_after_seconds: retry_after,
        });
    }

    // Increment counter
    let new_count: u32 = conn.incr(&window_key, 1).await.unwrap_or(1);
    
    // Set expiration for the window
    let _: () = conn.expire(&window_key, config.window_seconds as i64).await.unwrap_or(());

    let remaining = effective_limit.saturating_sub(new_count);
    let reset_time = window_start + config.window_seconds as u64;

    Ok(RateLimitInfo {
        limit: config.requests_per_window,
        remaining,
        reset_time,
    })
}

// Cleanup expired rate limit keys (should be called periodically)
pub async fn cleanup_rate_limit_keys(redis_pool: &deadpool_redis::Pool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut conn = redis_pool.get().await?;
    
    // Get all rate limit keys
    let keys: Vec<String> = conn.keys("rate_limit:*").await?;
    
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let mut expired_keys = Vec::new();
    
    for key in keys {
        // Extract timestamp from key
        if let Some(timestamp_str) = key.split(':').last() {
            if let Ok(timestamp) = timestamp_str.parse::<u64>() {
                // If window is older than 2 times the window size, consider it expired
                if now > timestamp + 120 { // 2 minutes buffer
                    expired_keys.push(key);
                }
            }
        }
    }

    if !expired_keys.is_empty() {
        let _: () = conn.del(&expired_keys).await?;
    }

    Ok(())
}