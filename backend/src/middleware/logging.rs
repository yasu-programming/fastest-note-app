use axum::{
    extract::{Request, ConnectInfo},
    http::{HeaderMap, Method, StatusCode, Uri},
    middleware::Next,
    response::Response,
};
use serde_json::json;
use std::{net::SocketAddr, time::Instant};
use tracing::{info, warn, error, debug};
use uuid::Uuid;

// Request logging middleware with performance tracking
pub async fn request_logging_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    mut req: Request,
    next: Next,
) -> Response {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();
    
    // Extract request information
    let method = req.method().clone();
    let uri = req.uri().clone();
    let version = req.version();
    let headers = req.headers().clone();
    let client_ip = get_client_ip(&headers, addr);
    
    // Add request ID to headers for tracking
    req.headers_mut().insert(
        "x-request-id",
        request_id.parse().unwrap_or_else(|_| "unknown".parse().unwrap())
    );

    // Log request start
    info!(
        request_id = %request_id,
        method = %method,
        uri = %uri,
        version = ?version,
        client_ip = %client_ip,
        user_agent = %headers.get("user-agent").and_then(|h| h.to_str().ok()).unwrap_or("unknown"),
        "Request started"
    );

    // Process request
    let response = next.run(req).await;
    
    // Calculate response time
    let duration = start_time.elapsed();
    let status = response.status();
    
    // Add response headers
    let mut response = response;
    response.headers_mut().insert(
        "x-request-id",
        request_id.parse().unwrap_or_else(|_| "unknown".parse().unwrap())
    );
    response.headers_mut().insert(
        "x-response-time",
        format!("{}ms", duration.as_millis()).parse().unwrap_or_else(|_| "unknown".parse().unwrap())
    );

    // Log response
    let log_level = match status.as_u16() {
        200..=299 => "info",
        300..=399 => "info", 
        400..=499 => "warn",
        500..=599 => "error",
        _ => "debug",
    };

    match log_level {
        "info" => info!(
            request_id = %request_id,
            method = %method,
            uri = %uri,
            status = %status,
            duration_ms = %duration.as_millis(),
            client_ip = %client_ip,
            "Request completed"
        ),
        "warn" => warn!(
            request_id = %request_id,
            method = %method,
            uri = %uri,
            status = %status,
            duration_ms = %duration.as_millis(),
            client_ip = %client_ip,
            "Request completed with client error"
        ),
        "error" => error!(
            request_id = %request_id,
            method = %method,
            uri = %uri,
            status = %status,
            duration_ms = %duration.as_millis(),
            client_ip = %client_ip,
            "Request completed with server error"
        ),
        _ => debug!(
            request_id = %request_id,
            method = %method,
            uri = %uri,
            status = %status,
            duration_ms = %duration.as_millis(),
            client_ip = %client_ip,
            "Request completed"
        ),
    }

    // Log slow requests
    if duration.as_millis() > 1000 {
        warn!(
            request_id = %request_id,
            method = %method,
            uri = %uri,
            duration_ms = %duration.as_millis(),
            "Slow request detected"
        );
    }

    response
}

// Structured logging middleware with JSON output
pub async fn structured_logging_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    mut req: Request,
    next: Next,
) -> Response {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();
    
    let method = req.method().clone();
    let uri = req.uri().clone();
    let headers = req.headers().clone();
    let client_ip = get_client_ip(&headers, addr);
    let user_agent = headers.get("user-agent")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown");

    // Add request ID to request
    req.headers_mut().insert(
        "x-request-id",
        request_id.parse().unwrap_or_else(|_| "unknown".parse().unwrap())
    );

    // Log structured request start
    let request_log = json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "level": "INFO",
        "event": "request_started",
        "request_id": request_id,
        "method": method.to_string(),
        "uri": uri.to_string(),
        "client_ip": client_ip,
        "user_agent": user_agent,
        "headers": {
            "content-type": headers.get("content-type").and_then(|h| h.to_str().ok()),
            "authorization": headers.get("authorization").map(|_| "[REDACTED]"),
            "accept": headers.get("accept").and_then(|h| h.to_str().ok()),
        }
    });

    println!("{}", request_log);

    let response = next.run(req).await;
    let duration = start_time.elapsed();
    let status = response.status();

    // Add response headers
    let mut response = response;
    response.headers_mut().insert(
        "x-request-id",
        request_id.parse().unwrap_or_else(|_| "unknown".parse().unwrap())
    );
    response.headers_mut().insert(
        "x-response-time",
        format!("{}ms", duration.as_millis()).parse().unwrap_or_else(|_| "unknown".parse().unwrap())
    );

    // Log structured response
    let response_log = json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "level": match status.as_u16() {
            200..=299 => "INFO",
            300..=399 => "INFO",
            400..=499 => "WARN", 
            500..=599 => "ERROR",
            _ => "DEBUG",
        },
        "event": "request_completed",
        "request_id": request_id,
        "method": method.to_string(),
        "uri": uri.to_string(),
        "status": status.as_u16(),
        "duration_ms": duration.as_millis(),
        "client_ip": client_ip,
        "response_size": response.headers()
            .get("content-length")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse::<usize>().ok()),
    });

    println!("{}", response_log);

    response
}

// Security logging middleware
pub async fn security_logging_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Response {
    let client_ip = get_client_ip(req.headers(), addr);
    let method = req.method();
    let uri = req.uri();
    let headers = req.headers();

    // Log suspicious activities
    let mut suspicious_indicators = Vec::new();

    // Check for potential SQL injection
    let query = uri.query().unwrap_or("");
    if query.to_lowercase().contains("select") || 
       query.to_lowercase().contains("union") || 
       query.to_lowercase().contains("drop") {
        suspicious_indicators.push("potential_sql_injection_in_query");
    }

    // Check for suspicious user agents
    if let Some(user_agent) = headers.get("user-agent").and_then(|h| h.to_str().ok()) {
        let user_agent_lower = user_agent.to_lowercase();
        if user_agent_lower.contains("sqlmap") || 
           user_agent_lower.contains("nmap") ||
           user_agent_lower.contains("burp") {
            suspicious_indicators.push("suspicious_user_agent");
        }
    }

    // Check for excessive header size
    let total_header_size: usize = headers.iter()
        .map(|(name, value)| name.as_str().len() + value.len())
        .sum();
    if total_header_size > 8192 { // 8KB limit
        suspicious_indicators.push("excessive_header_size");
    }

    // Log security events
    if !suspicious_indicators.is_empty() {
        warn!(
            client_ip = %client_ip,
            method = %method,
            uri = %uri,
            indicators = ?suspicious_indicators,
            user_agent = ?headers.get("user-agent").and_then(|h| h.to_str().ok()),
            "Suspicious request detected"
        );
    }

    let response = next.run(req).await;

    // Log failed authentication attempts
    if response.status() == StatusCode::UNAUTHORIZED {
        warn!(
            client_ip = %client_ip,
            method = %method,
            uri = %uri,
            "Authentication failed"
        );
    }

    // Log potential brute force attempts
    if uri.path().contains("/auth/login") && response.status() == StatusCode::UNAUTHORIZED {
        error!(
            client_ip = %client_ip,
            uri = %uri,
            "Failed login attempt"
        );
    }

    response
}

// Performance monitoring middleware
pub async fn performance_monitoring_middleware(
    req: Request,
    next: Next,
) -> Response {
    let start_time = Instant::now();
    let path = req.uri().path().to_string();
    let method = req.method().clone();

    let response = next.run(req).await;
    let duration = start_time.elapsed();
    let status = response.status();

    // Log performance metrics
    info!(
        method = %method,
        path = %path,
        status = %status,
        duration_ms = %duration.as_millis(),
        duration_ns = %duration.as_nanos(),
        "Performance metrics"
    );

    // Log performance alerts
    let alert_threshold = match path.as_str() {
        p if p.contains("/search") => 500,     // 500ms for search
        p if p.contains("/notes") => 200,     // 200ms for note operations
        p if p.contains("/folders") => 100,   // 100ms for folder operations
        p if p.contains("/auth") => 1000,     // 1s for auth operations
        _ => 300,                            // 300ms for other operations
    };

    if duration.as_millis() > alert_threshold {
        warn!(
            method = %method,
            path = %path,
            duration_ms = %duration.as_millis(),
            threshold_ms = %alert_threshold,
            "Performance threshold exceeded"
        );
    }

    response
}

// Helper function to extract real client IP
fn get_client_ip(headers: &HeaderMap, addr: SocketAddr) -> String {
    // Check for forwarded headers in order of precedence
    if let Some(forwarded) = headers.get("x-forwarded-for").and_then(|h| h.to_str().ok()) {
        if let Some(ip) = forwarded.split(',').next() {
            return ip.trim().to_string();
        }
    }

    if let Some(real_ip) = headers.get("x-real-ip").and_then(|h| h.to_str().ok()) {
        return real_ip.to_string();
    }

    if let Some(forwarded) = headers.get("forwarded").and_then(|h| h.to_str().ok()) {
        // Parse RFC 7239 Forwarded header
        for pair in forwarded.split(';') {
            if let Some(for_part) = pair.strip_prefix("for=") {
                if let Some(ip) = for_part.split(',').next() {
                    return ip.trim().trim_matches('"').to_string();
                }
            }
        }
    }

    addr.ip().to_string()
}

// Error logging middleware
pub async fn error_logging_middleware(
    req: Request,
    next: Next,
) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();

    let response = next.run(req).await;
    let status = response.status();

    // Log different types of errors with appropriate context
    match status.as_u16() {
        400 => debug!(
            method = %method,
            uri = %uri,
            status = %status,
            "Bad request"
        ),
        401 => info!(
            method = %method,
            uri = %uri,
            status = %status,
            "Unauthorized access attempt"
        ),
        403 => warn!(
            method = %method,
            uri = %uri,
            status = %status,
            "Forbidden access attempt"
        ),
        404 => debug!(
            method = %method,
            uri = %uri,
            status = %status,
            "Resource not found"
        ),
        429 => warn!(
            method = %method,
            uri = %uri,
            status = %status,
            "Rate limit exceeded"
        ),
        500..=599 => error!(
            method = %method,
            uri = %uri,
            status = %status,
            "Server error"
        ),
        _ => {}
    }

    response
}