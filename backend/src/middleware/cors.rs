use axum::{
    extract::Request,
    http::{
        header::{
            ACCEPT, AUTHORIZATION, CONTENT_TYPE, ORIGIN, ACCESS_CONTROL_ALLOW_ORIGIN,
            ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_HEADERS,
            ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_MAX_AGE,
            ACCESS_CONTROL_EXPOSE_HEADERS,
        },
        HeaderValue, Method, StatusCode,
    },
    middleware::Next,
    response::{IntoResponse, Response},
};

#[derive(Clone, Debug)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
    pub allowed_methods: Vec<Method>,
    pub allowed_headers: Vec<String>,
    pub exposed_headers: Vec<String>,
    pub allow_credentials: bool,
    pub max_age: Option<u64>,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec![
                "http://localhost:3000".to_string(),
                "http://127.0.0.1:3000".to_string(),
                "https://localhost:3000".to_string(),
            ],
            allowed_methods: vec![
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::PATCH,
                Method::OPTIONS,
            ],
            allowed_headers: vec![
                "content-type".to_string(),
                "authorization".to_string(),
                "accept".to_string(),
                "origin".to_string(),
                "x-requested-with".to_string(),
                "x-api-version".to_string(),
                "x-client-version".to_string(),
            ],
            exposed_headers: vec![
                "x-ratelimit-limit".to_string(),
                "x-ratelimit-remaining".to_string(),
                "x-ratelimit-reset".to_string(),
                "x-response-time".to_string(),
            ],
            allow_credentials: true,
            max_age: Some(86400), // 24 hours
        }
    }
}

impl CorsConfig {
    pub fn development() -> Self {
        Self {
            allowed_origins: vec!["*".to_string()],
            ..Default::default()
        }
    }

    pub fn production(allowed_domains: Vec<String>) -> Self {
        Self {
            allowed_origins: allowed_domains,
            ..Default::default()
        }
    }
}

pub async fn cors_middleware(
    config: CorsConfig,
    req: Request,
    next: Next,
) -> Response {
    let origin = req.headers().get(ORIGIN);
    let method = req.method();

    // Handle preflight requests
    if method == Method::OPTIONS {
        return handle_preflight(origin, &config);
    }

    // Handle actual requests
    let mut response = next.run(req).await;
    add_cors_headers(&mut response, origin, &config);
    response
}

fn handle_preflight(origin: Option<&HeaderValue>, config: &CorsConfig) -> Response {
    let mut response = Response::builder().status(StatusCode::NO_CONTENT);

    // Set Access-Control-Allow-Origin
    if let Some(origin_value) = get_allowed_origin(origin, config) {
        response = response.header(ACCESS_CONTROL_ALLOW_ORIGIN, origin_value);
    }

    // Set Access-Control-Allow-Methods
    let methods = config
        .allowed_methods
        .iter()
        .map(|m| m.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    response = response.header(ACCESS_CONTROL_ALLOW_METHODS, methods);

    // Set Access-Control-Allow-Headers
    let headers = config.allowed_headers.join(", ");
    response = response.header(ACCESS_CONTROL_ALLOW_HEADERS, headers);

    // Set Access-Control-Allow-Credentials
    if config.allow_credentials {
        response = response.header(ACCESS_CONTROL_ALLOW_CREDENTIALS, "true");
    }

    // Set Access-Control-Max-Age
    if let Some(max_age) = config.max_age {
        response = response.header(ACCESS_CONTROL_MAX_AGE, max_age.to_string());
    }

    response.body(axum::body::Body::empty()).unwrap()
}

fn add_cors_headers(response: &mut Response, origin: Option<&HeaderValue>, config: &CorsConfig) {
    let headers = response.headers_mut();

    // Set Access-Control-Allow-Origin
    if let Some(origin_value) = get_allowed_origin(origin, config) {
        headers.insert(ACCESS_CONTROL_ALLOW_ORIGIN, origin_value);
    }

    // Set Access-Control-Allow-Credentials
    if config.allow_credentials {
        headers.insert(
            ACCESS_CONTROL_ALLOW_CREDENTIALS,
            HeaderValue::from_static("true"),
        );
    }

    // Set Access-Control-Expose-Headers
    if !config.exposed_headers.is_empty() {
        let exposed = config.exposed_headers.join(", ");
        if let Ok(header_value) = HeaderValue::from_str(&exposed) {
            headers.insert(ACCESS_CONTROL_EXPOSE_HEADERS, header_value);
        }
    }
}

fn get_allowed_origin(origin: Option<&HeaderValue>, config: &CorsConfig) -> Option<HeaderValue> {
    let origin_str = origin?.to_str().ok()?;

    // Check if wildcard is allowed
    if config.allowed_origins.contains(&"*".to_string()) {
        return Some(HeaderValue::from_static("*"));
    }

    // Check if specific origin is allowed
    if config.allowed_origins.contains(&origin_str.to_string()) {
        return HeaderValue::from_str(origin_str).ok();
    }

    // Check for pattern matching (e.g., subdomains)
    for allowed in &config.allowed_origins {
        if is_origin_match(origin_str, allowed) {
            return HeaderValue::from_str(origin_str).ok();
        }
    }

    None
}

fn is_origin_match(origin: &str, pattern: &str) -> bool {
    // Simple pattern matching - extend as needed
    if pattern.starts_with("*.") {
        let domain = &pattern[2..];
        return origin.ends_with(domain) || origin == &domain[1..]; // Handle both subdomain and root domain
    }
    
    origin == pattern
}

// Convenience middleware creator
pub fn create_cors_middleware(config: CorsConfig) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>> + Clone {
    move |req: Request, next: Next| {
        let config = config.clone();
        Box::pin(async move { cors_middleware(config, req, next).await })
    }
}

// Security-focused CORS for production
pub async fn secure_cors_middleware(
    req: Request,
    next: Next,
) -> Response {
    let config = CorsConfig {
        allowed_origins: vec![
            std::env::var("FRONTEND_URL").unwrap_or_else(|_| "https://yourdomain.com".to_string()),
        ],
        allowed_methods: vec![
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
        ], // Exclude OPTIONS from allowed methods in production
        allowed_headers: vec![
            "content-type".to_string(),
            "authorization".to_string(),
        ], // Minimal headers for security
        exposed_headers: vec![], // Don't expose internal headers
        allow_credentials: true,
        max_age: Some(3600), // Shorter cache time
    };

    cors_middleware(config, req, next).await
}

// Development-friendly CORS
pub async fn dev_cors_middleware(
    req: Request,
    next: Next,
) -> Response {
    let config = CorsConfig::development();
    cors_middleware(config, req, next).await
}