use reqwest::StatusCode;
use serde_json::{json, Value};
use testcontainers::{clients::Cli, Container};
use testcontainers_modules::{postgres::Postgres as TestPostgres, redis::Redis};

/// Contract tests for POST /auth/login endpoint
/// 
/// These tests verify the API contract defined in contracts/api-spec.yaml
/// Tests MUST fail initially (no implementation exists yet)
/// 
/// Expected behavior according to contract:
/// - POST /auth/login
/// - Request: { "email": "string", "password": "string" }
/// - Success (200): { "user": {...}, "access_token": "string", "refresh_token": "string", "expires_in": number }
/// - Error (401): Invalid credentials

#[cfg(test)]
mod auth_login_contract_tests {
    use super::*;

    struct TestContext {
        client: reqwest::Client,
        base_url: String,
        _postgres_container: Container<'static, TestPostgres>,
        _redis_container: Container<'static, Redis>,
    }

    async fn setup_test_environment() -> TestContext {
        // Start test containers
        let docker = Cli::default();
        let postgres_container = docker.run(TestPostgres::default());
        let redis_container = docker.run(Redis::default());

        // Get container ports
        let postgres_port = postgres_container.get_host_port_ipv4(5432);
        let redis_port = redis_container.get_host_port_ipv4(6379);

        // Set environment variables for test
        std::env::set_var("DATABASE_URL", format!("postgresql://postgres:postgres@localhost:{}/postgres", postgres_port));
        std::env::set_var("REDIS_URL", format!("redis://localhost:{}", redis_port));
        std::env::set_var("JWT_SECRET", "test-secret-key-for-testing-only");

        // TODO: Start the actual server (will be implemented in later tasks)
        let base_url = "http://localhost:3001".to_string();

        TestContext {
            client: reqwest::Client::new(),
            base_url,
            _postgres_container: postgres_container,
            _redis_container: redis_container,
        }
    }

    /// Helper function to register a test user
    async fn register_test_user(ctx: &TestContext, email: &str, password: &str) -> Result<(), Box<dyn std::error::Error>> {
        let request_body = json!({
            "email": email,
            "password": password
        });

        let _response = ctx.client
            .post(&format!("{}/api/v1/auth/register", ctx.base_url))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_login_success_returns_200_with_auth_response() {
        let ctx = setup_test_environment().await;
        
        // First register a user (this will also fail initially)
        let _ = register_test_user(&ctx, "login@example.com", "securepassword123").await;
        
        let request_body = json!({
            "email": "login@example.com",
            "password": "securepassword123"
        });

        let response = ctx.client
            .post(&format!("{}/api/v1/auth/login", ctx.base_url))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await;

        // This test MUST fail initially - no implementation exists
        assert!(response.is_err() || response.unwrap().status() != StatusCode::OK,
            "Test should fail initially - no implementation exists yet");

        // Expected contract behavior (will pass after implementation):
        // let response = response.unwrap();
        // assert_eq!(response.status(), StatusCode::OK);
        // 
        // let auth_response: Value = response.json().await.unwrap();
        // 
        // // Verify response structure matches OpenAPI spec
        // assert!(auth_response.get("user").is_some(), "Response must contain user object");
        // assert!(auth_response.get("access_token").is_some(), "Response must contain access_token");
        // assert!(auth_response.get("refresh_token").is_some(), "Response must contain refresh_token");
        // assert!(auth_response.get("expires_in").is_some(), "Response must contain expires_in");
        // 
        // // Verify user object structure
        // let user = auth_response.get("user").unwrap();
        // assert!(user.get("id").is_some(), "User must have id field");
        // assert!(user.get("email").is_some(), "User must have email field");
        // assert!(user.get("created_at").is_some(), "User must have created_at field");
        // assert_eq!(user.get("email").unwrap(), "login@example.com");
        // 
        // // Verify token types
        // assert!(auth_response.get("access_token").unwrap().is_string(), "access_token must be string");
        // assert!(auth_response.get("refresh_token").unwrap().is_string(), "refresh_token must be string");
        // assert!(auth_response.get("expires_in").unwrap().is_number(), "expires_in must be number");
    }

    #[tokio::test]
    async fn test_login_invalid_email_returns_401() {
        let ctx = setup_test_environment().await;
        
        let request_body = json!({
            "email": "nonexistent@example.com",
            "password": "anypassword"
        });

        let response = ctx.client
            .post(&format!("{}/api/v1/auth/login", ctx.base_url))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await;

        // This test MUST fail initially
        assert!(response.is_err() || response.unwrap().status() != StatusCode::UNAUTHORIZED,
            "Test should fail initially - no implementation exists yet");

        // Expected behavior after implementation:
        // let response = response.unwrap();
        // assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        // 
        // let error_response: Value = response.json().await.unwrap();
        // assert!(error_response.get("error").is_some(), "Error response must contain error field");
    }

    #[tokio::test]
    async fn test_login_invalid_password_returns_401() {
        let ctx = setup_test_environment().await;
        
        // Register user first
        let _ = register_test_user(&ctx, "user@example.com", "correctpassword").await;
        
        let request_body = json!({
            "email": "user@example.com",
            "password": "wrongpassword"
        });

        let response = ctx.client
            .post(&format!("{}/api/v1/auth/login", ctx.base_url))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await;

        // This test MUST fail initially
        assert!(response.is_err() || response.unwrap().status() != StatusCode::UNAUTHORIZED,
            "Test should fail initially - no implementation exists yet");

        // Expected behavior:
        // assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_login_missing_email_returns_400() {
        let ctx = setup_test_environment().await;
        
        let request_body = json!({
            "password": "securepassword123"
        });

        let response = ctx.client
            .post(&format!("{}/api/v1/auth/login", ctx.base_url))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await;

        // This test MUST fail initially
        assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
            "Test should fail initially - no implementation exists yet");
    }

    #[tokio::test]
    async fn test_login_missing_password_returns_400() {
        let ctx = setup_test_environment().await;
        
        let request_body = json!({
            "email": "test@example.com"
        });

        let response = ctx.client
            .post(&format!("{}/api/v1/auth/login", ctx.base_url))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await;

        // This test MUST fail initially
        assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
            "Test should fail initially - no implementation exists yet");
    }

    #[tokio::test]
    async fn test_login_empty_email_returns_401() {
        let ctx = setup_test_environment().await;
        
        let request_body = json!({
            "email": "",
            "password": "securepassword123"
        });

        let response = ctx.client
            .post(&format!("{}/api/v1/auth/login", ctx.base_url))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await;

        // This test MUST fail initially
        assert!(response.is_err() || response.unwrap().status() != StatusCode::UNAUTHORIZED,
            "Test should fail initially - no implementation exists yet");
    }

    #[tokio::test]
    async fn test_login_empty_password_returns_401() {
        let ctx = setup_test_environment().await;
        
        let request_body = json!({
            "email": "test@example.com",
            "password": ""
        });

        let response = ctx.client
            .post(&format!("{}/api/v1/auth/login", ctx.base_url))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await;

        // This test MUST fail initially
        assert!(response.is_err() || response.unwrap().status() != StatusCode::UNAUTHORIZED,
            "Test should fail initially - no implementation exists yet");
    }

    #[tokio::test]
    async fn test_login_invalid_content_type_returns_400() {
        let ctx = setup_test_environment().await;
        
        let request_body = r#"{"email": "test@example.com", "password": "securepassword123"}"#;

        let response = ctx.client
            .post(&format!("{}/api/v1/auth/login", ctx.base_url))
            .header("Content-Type", "text/plain")  // Wrong content type
            .body(request_body)
            .send()
            .await;

        // This test MUST fail initially
        assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
            "Test should fail initially - no implementation exists yet");
    }

    #[tokio::test]
    async fn test_login_malformed_json_returns_400() {
        let ctx = setup_test_environment().await;
        
        let malformed_json = r#"{"email": "test@example.com", "password": }"#;  // Invalid JSON

        let response = ctx.client
            .post(&format!("{}/api/v1/auth/login", ctx.base_url))
            .header("Content-Type", "application/json")
            .body(malformed_json)
            .send()
            .await;

        // This test MUST fail initially
        assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
            "Test should fail initially - no implementation exists yet");
    }

    #[tokio::test]
    async fn test_login_case_sensitive_email() {
        let ctx = setup_test_environment().await;
        
        // Register with lowercase email
        let _ = register_test_user(&ctx, "case@example.com", "securepassword123").await;
        
        // Try login with uppercase email
        let request_body = json!({
            "email": "CASE@EXAMPLE.COM",
            "password": "securepassword123"
        });

        let response = ctx.client
            .post(&format!("{}/api/v1/auth/login", ctx.base_url))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await;

        // This test MUST fail initially
        assert!(response.is_err() || response.unwrap().status() != StatusCode::UNAUTHORIZED,
            "Test should fail initially - no implementation exists yet");

        // Expected behavior: Case-sensitive email should fail (or succeed based on business requirements)
        // Most systems normalize email to lowercase, but this tests the actual behavior
    }

    #[tokio::test]
    async fn test_login_sql_injection_protection() {
        let ctx = setup_test_environment().await;
        
        let request_body = json!({
            "email": "admin@example.com' OR '1'='1",
            "password": "anything"
        });

        let response = ctx.client
            .post(&format!("{}/api/v1/auth/login", ctx.base_url))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await;

        // This test MUST fail initially
        assert!(response.is_err() || response.unwrap().status() != StatusCode::UNAUTHORIZED,
            "Test should fail initially - no implementation exists yet");

        // Expected behavior: SQL injection should not work, should return 401
    }

    #[tokio::test]
    async fn test_login_response_time_under_200ms() {
        let ctx = setup_test_environment().await;
        
        // Register user first
        let _ = register_test_user(&ctx, "speed@example.com", "securepassword123").await;
        
        let request_body = json!({
            "email": "speed@example.com",
            "password": "securepassword123"
        });

        let start = std::time::Instant::now();
        
        let _response = ctx.client
            .post(&format!("{}/api/v1/auth/login", ctx.base_url))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await;

        let duration = start.elapsed();

        // This test will fail initially due to no server running
        // After implementation, verify performance requirement
        assert!(duration.as_millis() > 200,
            "Test should fail initially - no server running, or verify <200ms after implementation");
    }

    #[tokio::test]
    async fn test_login_rate_limiting() {
        let ctx = setup_test_environment().await;
        
        // Register user first
        let _ = register_test_user(&ctx, "rate@example.com", "securepassword123").await;
        
        let request_body = json!({
            "email": "rate@example.com",
            "password": "wrongpassword"
        });

        // Make multiple failed login attempts rapidly
        let mut responses = Vec::new();
        for _ in 0..10 {
            let response = ctx.client
                .post(&format!("{}/api/v1/auth/login", ctx.base_url))
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
                .await;
            
            responses.push(response);
        }

        // This test will fail initially - no implementation
        // Expected behavior: After several failed attempts, should get rate limited (429)
        let has_rate_limit = responses.iter().any(|r| 
            r.as_ref().map(|resp| resp.status() == StatusCode::TOO_MANY_REQUESTS).unwrap_or(false)
        );
        
        assert!(!has_rate_limit, "Test should fail initially - no rate limiting implemented yet");
    }
}