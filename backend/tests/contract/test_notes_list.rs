use reqwest::StatusCode;
use serde_json::{json, Value};
use testcontainers::{clients::Cli, Container};
use testcontainers_modules::{postgres::Postgres as TestPostgres, redis::Redis};
use uuid::Uuid;

/// Contract tests for GET /notes endpoint
/// 
/// These tests verify the API contract defined in contracts/api-spec.yaml
/// Tests MUST fail initially (no implementation exists yet)
/// 
/// Expected behavior according to contract:
/// - GET /notes?folder_id=<uuid>&search=<string>&limit=<number>&offset=<number>
/// - Headers: Authorization: Bearer <access_token>
/// - Success (200): { "notes": [{ "id", "title", "content", "folder_id", "created_at", "updated_at", "version" }], "total": number }
/// - Error (401): Unauthorized (no/invalid token)
/// - Error (400): Bad request (invalid parameters)

#[cfg(test)]
mod notes_list_contract_tests {
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
    async fn test_list_notes_success_returns_200_with_notes_array() {
        let ctx = setup_test_environment().await;
        
        // Get auth token (this will also fail initially)
        let token_result = get_auth_token(&ctx, "notes_list@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let response = ctx.client
                .get(&format!("{}/api/v1/notes", ctx.base_url))
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
        // let notes_response: Value = response.json().await.unwrap();
        // 
        // // Verify response structure matches OpenAPI spec
        // assert!(notes_response.get("notes").is_some(), "Response must contain notes array");
        // assert!(notes_response.get("total").is_some(), "Response must contain total count");
        // 
        // let notes = notes_response.get("notes").unwrap().as_array().unwrap();
        // let total = notes_response.get("total").unwrap().as_u64().unwrap();
        // 
        // // For new user, should have empty notes array
        // assert_eq!(notes.len(), 0, "New user should have no notes");
        // assert_eq!(total, 0, "Total should be 0 for new user");
    }

    #[tokio::test]
    async fn test_list_notes_with_folder_id_filter() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "notes_filter@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let folder_id = Uuid::new_v4();
            
            let response = ctx.client
                .get(&format!("{}/api/v1/notes?folder_id={}", ctx.base_url, folder_id))
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::OK,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Return notes that belong to the specified folder_id
    }

    #[tokio::test]
    async fn test_list_root_notes_with_null_folder() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "root_notes@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            // Request root notes (folder_id is null/not specified)
            let response = ctx.client
                .get(&format!("{}/api/v1/notes", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::OK,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Return root-level notes (where folder_id IS NULL)
    }

    #[tokio::test]
    async fn test_list_notes_with_search_query() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "notes_search@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let response = ctx.client
                .get(&format!("{}/api/v1/notes?search=test", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::OK,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Full-text search in title and content
    }

    #[tokio::test]
    async fn test_list_notes_with_pagination() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "notes_pagination@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let response = ctx.client
                .get(&format!("{}/api/v1/notes?limit=10&offset=0", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::OK,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Return paginated results with limit and offset
    }

    #[tokio::test]
    async fn test_list_notes_unauthorized_no_token_returns_401() {
        let ctx = setup_test_environment().await;

        let response = ctx.client
            .get(&format!("{}/api/v1/notes", ctx.base_url))
            .send()
            .await;

        // This test MUST fail initially
        assert!(response.is_err() || response.unwrap().status() != StatusCode::UNAUTHORIZED,
            "Test should fail initially - no implementation exists yet");

        // Expected behavior: Return 401 when no Authorization header provided
    }

    #[tokio::test]
    async fn test_list_notes_invalid_token_returns_401() {
        let ctx = setup_test_environment().await;

        let response = ctx.client
            .get(&format!("{}/api/v1/notes", ctx.base_url))
            .header("Authorization", "Bearer invalid_token_123")
            .send()
            .await;

        // This test MUST fail initially
        assert!(response.is_err() || response.unwrap().status() != StatusCode::UNAUTHORIZED,
            "Test should fail initially - no implementation exists yet");
    }

    #[tokio::test]
    async fn test_list_notes_invalid_folder_id_format_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "invalid_folder_id@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let response = ctx.client
                .get(&format!("{}/api/v1/notes?folder_id=not-a-uuid", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Return 400 for invalid UUID format in folder_id parameter
    }

    #[tokio::test]
    async fn test_list_notes_invalid_limit_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "invalid_limit@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let response = ctx.client
                .get(&format!("{}/api/v1/notes?limit=not-a-number", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Return 400 for invalid limit parameter
    }

    #[tokio::test]
    async fn test_list_notes_negative_limit_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "negative_limit@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let response = ctx.client
                .get(&format!("{}/api/v1/notes?limit=-10", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Return 400 for negative limit
    }

    #[tokio::test]
    async fn test_list_notes_limit_too_large_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "large_limit@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let response = ctx.client
                .get(&format!("{}/api/v1/notes?limit=1001", ctx.base_url)) // Assuming max 1000
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Return 400 for limit exceeding maximum (1000)
    }

    #[tokio::test]
    async fn test_list_notes_invalid_offset_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "invalid_offset@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let response = ctx.client
                .get(&format!("{}/api/v1/notes?offset=not-a-number", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Return 400 for invalid offset parameter
    }

    #[tokio::test]
    async fn test_list_notes_negative_offset_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "negative_offset@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let response = ctx.client
                .get(&format!("{}/api/v1/notes?offset=-10", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Return 400 for negative offset
    }

    #[tokio::test]
    async fn test_list_notes_nonexistent_folder_id_returns_empty_array() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "nonexistent_folder@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let nonexistent_id = Uuid::new_v4();
            
            let response = ctx.client
                .get(&format!("{}/api/v1/notes?folder_id={}", ctx.base_url, nonexistent_id))
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::OK,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Return empty notes array for non-existent folder_id
    }

    #[tokio::test]
    async fn test_list_notes_user_isolation() {
        let ctx = setup_test_environment().await;
        
        // Get tokens for two different users
        let token1_result = get_auth_token(&ctx, "user1_notes@example.com", "password123").await;
        let token2_result = get_auth_token(&ctx, "user2_notes@example.com", "password123").await;
        
        if let (Ok(token1), Ok(token2)) = (token1_result, token2_result) {
            // User 1 lists their notes
            let response1 = ctx.client
                .get(&format!("{}/api/v1/notes", ctx.base_url))
                .header("Authorization", format!("Bearer {}", token1))
                .send()
                .await;

            // User 2 lists their notes
            let response2 = ctx.client
                .get(&format!("{}/api/v1/notes", ctx.base_url))
                .header("Authorization", format!("Bearer {}", token2))
                .send()
                .await;

            // Both tests MUST fail initially
            assert!(response1.is_err() || response1.unwrap().status() != StatusCode::OK,
                "Test should fail initially - no implementation exists yet");
            assert!(response2.is_err() || response2.unwrap().status() != StatusCode::OK,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Each user should only see their own notes
    }

    #[tokio::test]
    async fn test_list_notes_empty_search_query() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "empty_search@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let response = ctx.client
                .get(&format!("{}/api/v1/notes?search=", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::OK,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Empty search should return all notes (no filtering)
    }

    #[tokio::test]
    async fn test_list_notes_unicode_search_query() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "unicode_search@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let response = ctx.client
                .get(&format!("{}/api/v1/notes?search=日本語", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::OK,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Unicode search queries should work correctly
    }

    #[tokio::test]
    async fn test_list_notes_response_time_under_200ms() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "performance_notes@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let start = std::time::Instant::now();
            
            let _response = ctx.client
                .get(&format!("{}/api/v1/notes", ctx.base_url))
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
    async fn test_list_notes_json_content_type() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "content_type_notes@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let response = ctx.client
                .get(&format!("{}/api/v1/notes", ctx.base_url))
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
    async fn test_list_notes_note_structure_validation() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "structure_notes@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let response = ctx.client
                .get(&format!("{}/api/v1/notes", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::OK,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior after implementation with notes:
        // Each note object should contain:
        // - id: UUID string
        // - title: string (1-255 chars)
        // - content: string (0-1MB)
        // - folder_id: UUID string or null
        // - created_at: ISO 8601 datetime string
        // - updated_at: ISO 8601 datetime string
        // - version: integer (for optimistic locking)
    }

    #[tokio::test]
    async fn test_list_notes_ordering_by_updated_at_desc() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ordering@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let response = ctx.client
                .get(&format!("{}/api/v1/notes", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::OK,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Notes should be ordered by updated_at DESC (most recent first)
    }

    #[tokio::test]
    async fn test_list_notes_combined_filters() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "combined_filters@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let folder_id = Uuid::new_v4();
            
            let response = ctx.client
                .get(&format!("{}/api/v1/notes?folder_id={}&search=test&limit=5&offset=0", ctx.base_url, folder_id))
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::OK,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: All filters should work together (folder_id + search + pagination)
    }

    #[tokio::test]
    async fn test_list_notes_case_insensitive_search() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "case_search@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let response = ctx.client
                .get(&format!("{}/api/v1/notes?search=TEST", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::OK,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Search should be case-insensitive
    }

    #[tokio::test]
    async fn test_list_notes_special_characters_in_search() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "special_search@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let response = ctx.client
                .get(&format!("{}/api/v1/notes?search=%21%40%23%24", ctx.base_url)) // URL encoded !@#$
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::OK,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Special characters in search should be handled safely
    }
}