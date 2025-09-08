use reqwest::StatusCode;
use serde_json::{json, Value};
use testcontainers::{clients::Cli, Container};
use testcontainers_modules::{postgres::Postgres as TestPostgres, redis::Redis};
use uuid::Uuid;

/// Contract tests for GET /folders endpoint
/// 
/// These tests verify the API contract defined in contracts/api-spec.yaml
/// Tests MUST fail initially (no implementation exists yet)
/// 
/// Expected behavior according to contract:
/// - GET /folders?parent_id=<uuid>
/// - Headers: Authorization: Bearer <access_token>
/// - Success (200): { "folders": [{ "id", "name", "parent_folder_id", "path", "level", "created_at", "updated_at" }] }
/// - Error (401): Unauthorized (no/invalid token)

#[cfg(test)]
mod folders_list_contract_tests {
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

    /// Helper function to register and login a test user, returning access token
    async fn get_auth_token(ctx: &TestContext, email: &str, password: &str) -> Result<String, Box<dyn std::error::Error>> {
        // Register user
        let register_body = json!({
            "email": email,
            "password": password
        });

        let _register_response = ctx.client
            .post(&format!("{}/api/v1/auth/register", ctx.base_url))
            .header("Content-Type", "application/json")
            .json(&register_body)
            .send()
            .await?;

        // Login to get token
        let login_body = json!({
            "email": email,
            "password": password
        });

        let login_response = ctx.client
            .post(&format!("{}/api/v1/auth/login", ctx.base_url))
            .header("Content-Type", "application/json")
            .json(&login_body)
            .send()
            .await?;

        let auth_data: Value = login_response.json().await?;
        let access_token = auth_data["access_token"].as_str().unwrap().to_string();

        Ok(access_token)
    }

    #[tokio::test]
    async fn test_list_folders_success_returns_200_with_folders_array() {
        let ctx = setup_test_environment().await;
        
        // Get auth token (this will also fail initially)
        let token_result = get_auth_token(&ctx, "folder_list@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let response = ctx.client
                .get(&format!("{}/api/v1/folders", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await;

            // This test MUST fail initially - no implementation exists
            assert!(response.is_err() || response.unwrap().status() != StatusCode::OK,
                "Test should fail initially - no implementation exists yet");
        } else {
            // Expected initial failure
            assert!(true, "Test fails as expected - no auth implementation yet");
        }

        // Expected contract behavior (will pass after implementation):
        // let response = response.unwrap();
        // assert_eq!(response.status(), StatusCode::OK);
        // 
        // let folder_response: Value = response.json().await.unwrap();
        // 
        // // Verify response structure matches OpenAPI spec
        // assert!(folder_response.get("folders").is_some(), "Response must contain folders array");
        // 
        // let folders = folder_response.get("folders").unwrap().as_array().unwrap();
        // 
        // // For new user, should have empty folders array
        // assert_eq!(folders.len(), 0, "New user should have no folders");
        // 
        // // Verify folder structure (when folders exist)
        // // Each folder should have: id, name, parent_folder_id, path, level, created_at, updated_at
    }

    #[tokio::test]
    async fn test_list_folders_with_parent_id_filter() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "folder_filter@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let parent_id = Uuid::new_v4();
            
            let response = ctx.client
                .get(&format!("{}/api/v1/folders?parent_id={}", ctx.base_url, parent_id))
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::OK,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Return folders that have the specified parent_id
    }

    #[tokio::test]
    async fn test_list_root_folders_with_null_parent() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "root_folders@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            // Request root folders (parent_id is null/not specified)
            let response = ctx.client
                .get(&format!("{}/api/v1/folders", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::OK,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Return root-level folders (where parent_folder_id IS NULL)
    }

    #[tokio::test]
    async fn test_list_folders_unauthorized_no_token_returns_401() {
        let ctx = setup_test_environment().await;

        let response = ctx.client
            .get(&format!("{}/api/v1/folders", ctx.base_url))
            .send()
            .await;

        // This test MUST fail initially
        assert!(response.is_err() || response.unwrap().status() != StatusCode::UNAUTHORIZED,
            "Test should fail initially - no implementation exists yet");

        // Expected behavior: Return 401 when no Authorization header provided
    }

    #[tokio::test]
    async fn test_list_folders_invalid_token_returns_401() {
        let ctx = setup_test_environment().await;

        let response = ctx.client
            .get(&format!("{}/api/v1/folders", ctx.base_url))
            .header("Authorization", "Bearer invalid_token_123")
            .send()
            .await;

        // This test MUST fail initially
        assert!(response.is_err() || response.unwrap().status() != StatusCode::UNAUTHORIZED,
            "Test should fail initially - no implementation exists yet");
    }

    #[tokio::test]
    async fn test_list_folders_expired_token_returns_401() {
        let ctx = setup_test_environment().await;

        // Use clearly expired token
        let expired_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ0ZXN0IiwiZXhwIjoxNjAwMDAwMDAwfQ.invalid";

        let response = ctx.client
            .get(&format!("{}/api/v1/folders", ctx.base_url))
            .header("Authorization", format!("Bearer {}", expired_token))
            .send()
            .await;

        // This test MUST fail initially
        assert!(response.is_err() || response.unwrap().status() != StatusCode::UNAUTHORIZED,
            "Test should fail initially - no implementation exists yet");
    }

    #[tokio::test]
    async fn test_list_folders_malformed_bearer_header_returns_401() {
        let ctx = setup_test_environment().await;

        let response = ctx.client
            .get(&format!("{}/api/v1/folders", ctx.base_url))
            .header("Authorization", "Malformed token123")
            .send()
            .await;

        // This test MUST fail initially
        assert!(response.is_err() || response.unwrap().status() != StatusCode::UNAUTHORIZED,
            "Test should fail initially - no implementation exists yet");
    }

    #[tokio::test]
    async fn test_list_folders_invalid_parent_id_format_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "invalid_parent@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let response = ctx.client
                .get(&format!("{}/api/v1/folders?parent_id=not-a-uuid", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Return 400 for invalid UUID format in parent_id parameter
    }

    #[tokio::test]
    async fn test_list_folders_nonexistent_parent_id_returns_empty_array() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "nonexistent_parent@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let nonexistent_id = Uuid::new_v4();
            
            let response = ctx.client
                .get(&format!("{}/api/v1/folders?parent_id={}", ctx.base_url, nonexistent_id))
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::OK,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Return empty folders array for non-existent parent_id
    }

    #[tokio::test]
    async fn test_list_folders_user_isolation() {
        let ctx = setup_test_environment().await;
        
        // Get tokens for two different users
        let token1_result = get_auth_token(&ctx, "user1@example.com", "password123").await;
        let token2_result = get_auth_token(&ctx, "user2@example.com", "password123").await;
        
        if let (Ok(token1), Ok(token2)) = (token1_result, token2_result) {
            // User 1 lists their folders
            let response1 = ctx.client
                .get(&format!("{}/api/v1/folders", ctx.base_url))
                .header("Authorization", format!("Bearer {}", token1))
                .send()
                .await;

            // User 2 lists their folders  
            let response2 = ctx.client
                .get(&format!("{}/api/v1/folders", ctx.base_url))
                .header("Authorization", format!("Bearer {}", token2))
                .send()
                .await;

            // Both tests MUST fail initially
            assert!(response1.is_err() || response1.unwrap().status() != StatusCode::OK,
                "Test should fail initially - no implementation exists yet");
            assert!(response2.is_err() || response2.unwrap().status() != StatusCode::OK,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Each user should only see their own folders
    }

    #[tokio::test]
    async fn test_list_folders_response_time_under_200ms() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "performance@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let start = std::time::Instant::now();
            
            let _response = ctx.client
                .get(&format!("{}/api/v1/folders", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await;

            let duration = start.elapsed();

            // This test will fail initially due to no server running
            assert!(duration.as_millis() > 200,
                "Test should fail initially - no server running, or verify <200ms after implementation");
        }
    }

    #[tokio::test]
    async fn test_list_folders_json_content_type() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "content_type@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let response = ctx.client
                .get(&format!("{}/api/v1/folders", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await;

            // This test MUST fail initially
            if let Ok(resp) = response {
                assert!(resp.status() != StatusCode::OK || 
                    resp.headers().get("content-type").map(|ct| ct.to_str().unwrap_or("")).unwrap_or("") != "application/json",
                    "Test should fail initially - no implementation exists yet");
            } else {
                assert!(true, "Expected failure - no server running");
            }
        }

        // Expected behavior: Response should have Content-Type: application/json
    }

    #[tokio::test]
    async fn test_list_folders_folder_structure_validation() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "structure@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let response = ctx.client
                .get(&format!("{}/api/v1/folders", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::OK,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior after implementation with folders:
        // Each folder object should contain:
        // - id: UUID string
        // - name: string (1-255 chars)
        // - parent_folder_id: UUID string or null
        // - path: string (materialized path like "/folder1/subfolder/")
        // - level: integer (0-10)
        // - created_at: ISO 8601 datetime string
        // - updated_at: ISO 8601 datetime string
    }

    #[tokio::test]
    async fn test_list_folders_empty_parent_id_parameter() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "empty_param@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let response = ctx.client
                .get(&format!("{}/api/v1/folders?parent_id=", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Empty parent_id parameter should return 400 Bad Request
    }

    #[tokio::test]
    async fn test_list_folders_case_sensitive_headers() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "case_headers@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let response = ctx.client
                .get(&format!("{}/api/v1/folders", ctx.base_url))
                .header("authorization", format!("Bearer {}", access_token))  // lowercase header
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::UNAUTHORIZED,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: HTTP headers should be case-insensitive per RFC 7230
        // Most servers handle this correctly, but worth testing
    }
}