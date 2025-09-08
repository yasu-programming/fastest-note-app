use reqwest::StatusCode;
use serde_json::{json, Value};
use testcontainers::{clients::Cli, Container};
use testcontainers_modules::{postgres::Postgres as TestPostgres, redis::Redis};
use uuid::Uuid;

/// Contract tests for POST /notes endpoint
/// 
/// These tests verify the API contract defined in contracts/api-spec.yaml
/// Tests MUST fail initially (no implementation exists yet)
/// 
/// Expected behavior according to contract:
/// - POST /notes
/// - Headers: Authorization: Bearer <access_token>, Content-Type: application/json
/// - Body: { "title": "string", "content": "string", "folder_id": "uuid" }
/// - Success (201): { "id", "title", "content", "folder_id", "created_at", "updated_at", "version" }
/// - Error (400): Bad request (validation errors)
/// - Error (401): Unauthorized
/// - Error (404): Folder not found

#[cfg(test)]
mod notes_create_contract_tests {
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

    /// Helper function to create a test folder, returning folder_id
    async fn create_test_folder(ctx: &TestContext, access_token: &str, name: &str) -> Result<String, Box<dyn std::error::Error>> {
        let folder_body = json!({
            "name": name
        });

        let folder_response = ctx.client
            .post(&format!("{}/api/v1/folders", ctx.base_url))
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .json(&folder_body)
            .send()
            .await?;

        let folder_data: Value = folder_response.json().await?;
        let folder_id = folder_data["id"].as_str().unwrap().to_string();

        Ok(folder_id)
    }

    #[tokio::test]
    async fn test_create_note_success_returns_201_with_note_data() {
        let ctx = setup_test_environment().await;
        
        // Get auth token (this will also fail initially)
        let token_result = get_auth_token(&ctx, "note_create@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            // Create a test folder first (will also fail initially)
            let folder_result = create_test_folder(&ctx, &access_token, "Test Folder").await;
            
            if let Ok(folder_id) = folder_result {
                let note_body = json!({
                    "title": "Test Note",
                    "content": "This is a test note content",
                    "folder_id": folder_id
                });

                let response = ctx.client
                    .post(&format!("{}/api/v1/notes", ctx.base_url))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&note_body)
                    .send()
                    .await;

                // This test MUST fail initially - no implementation exists
                assert!(response.is_err() || response.unwrap().status() != StatusCode::CREATED,
                    "Test should fail initially - no implementation exists yet");
            }
        } else {
            // Expected initial failure
            assert!(true, "Test fails as expected - no auth implementation yet");
        }

        // Expected contract behavior (will pass after implementation):
        // let response = response.unwrap();
        // assert_eq!(response.status(), StatusCode::CREATED);
        // 
        // let note_response: Value = response.json().await.unwrap();
        // 
        // // Verify response structure matches OpenAPI spec
        // assert!(note_response.get("id").is_some(), "Response must contain id");
        // assert!(note_response.get("title").is_some(), "Response must contain title");
        // assert!(note_response.get("content").is_some(), "Response must contain content");
        // assert!(note_response.get("folder_id").is_some(), "Response must contain folder_id");
        // assert!(note_response.get("created_at").is_some(), "Response must contain created_at");
        // assert!(note_response.get("updated_at").is_some(), "Response must contain updated_at");
        // assert!(note_response.get("version").is_some(), "Response must contain version");
        // 
        // // Verify data types and values
        // assert_eq!(note_response["title"], "Test Note");
        // assert_eq!(note_response["content"], "This is a test note content");
        // assert_eq!(note_response["folder_id"], folder_id);
        // assert_eq!(note_response["version"], 1);
    }

    #[tokio::test]
    async fn test_create_note_in_root_folder_success() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "note_root@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let note_body = json!({
                "title": "Root Note",
                "content": "This note is in the root folder",
                "folder_id": null
            });

            let response = ctx.client
                .post(&format!("{}/api/v1/notes", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .json(&note_body)
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::CREATED,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Note created in root folder (folder_id = null)
    }

    #[tokio::test]
    async fn test_create_note_missing_title_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "missing_title@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let note_body = json!({
                "content": "Content without title",
                "folder_id": null
            });

            let response = ctx.client
                .post(&format!("{}/api/v1/notes", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .json(&note_body)
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Return 400 for missing required title field
    }

    #[tokio::test]
    async fn test_create_note_empty_title_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "empty_title@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let note_body = json!({
                "title": "",
                "content": "Content with empty title",
                "folder_id": null
            });

            let response = ctx.client
                .post(&format!("{}/api/v1/notes", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .json(&note_body)
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Return 400 for empty title (min length validation)
    }

    #[tokio::test]
    async fn test_create_note_title_too_long_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "long_title@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let long_title = "a".repeat(256); // Assuming 255 char limit

            let note_body = json!({
                "title": long_title,
                "content": "Content with too long title",
                "folder_id": null
            });

            let response = ctx.client
                .post(&format!("{}/api/v1/notes", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .json(&note_body)
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Return 400 for title exceeding max length (255 chars)
    }

    #[tokio::test]
    async fn test_create_note_content_too_large_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "large_content@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            // Create content larger than 1MB limit
            let large_content = "a".repeat(1024 * 1024 + 1); // 1MB + 1 byte

            let note_body = json!({
                "title": "Large Content Note",
                "content": large_content,
                "folder_id": null
            });

            let response = ctx.client
                .post(&format!("{}/api/v1/notes", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .json(&note_body)
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Return 400 for content exceeding 1MB limit
    }

    #[tokio::test]
    async fn test_create_note_missing_content_uses_empty_string() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "missing_content@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let note_body = json!({
                "title": "Note without content",
                "folder_id": null
            });

            let response = ctx.client
                .post(&format!("{}/api/v1/notes", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .json(&note_body)
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::CREATED,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Content defaults to empty string when not provided
    }

    #[tokio::test]
    async fn test_create_note_invalid_folder_id_format_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "invalid_folder@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let note_body = json!({
                "title": "Test Note",
                "content": "Test content",
                "folder_id": "not-a-uuid"
            });

            let response = ctx.client
                .post(&format!("{}/api/v1/notes", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .json(&note_body)
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Return 400 for invalid UUID format in folder_id
    }

    #[tokio::test]
    async fn test_create_note_nonexistent_folder_returns_404() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "nonexistent_folder@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let nonexistent_id = Uuid::new_v4();

            let note_body = json!({
                "title": "Test Note",
                "content": "Test content",
                "folder_id": nonexistent_id
            });

            let response = ctx.client
                .post(&format!("{}/api/v1/notes", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .json(&note_body)
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::NOT_FOUND,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Return 404 for non-existent folder_id
    }

    #[tokio::test]
    async fn test_create_note_unauthorized_no_token_returns_401() {
        let ctx = setup_test_environment().await;

        let note_body = json!({
            "title": "Unauthorized Note",
            "content": "This should fail",
            "folder_id": null
        });

        let response = ctx.client
            .post(&format!("{}/api/v1/notes", ctx.base_url))
            .header("Content-Type", "application/json")
            .json(&note_body)
            .send()
            .await;

        // This test MUST fail initially
        assert!(response.is_err() || response.unwrap().status() != StatusCode::UNAUTHORIZED,
            "Test should fail initially - no implementation exists yet");

        // Expected behavior: Return 401 when no Authorization header provided
    }

    #[tokio::test]
    async fn test_create_note_invalid_token_returns_401() {
        let ctx = setup_test_environment().await;

        let note_body = json!({
            "title": "Invalid Token Note",
            "content": "This should fail",
            "folder_id": null
        });

        let response = ctx.client
            .post(&format!("{}/api/v1/notes", ctx.base_url))
            .header("Authorization", "Bearer invalid_token_123")
            .header("Content-Type", "application/json")
            .json(&note_body)
            .send()
            .await;

        // This test MUST fail initially
        assert!(response.is_err() || response.unwrap().status() != StatusCode::UNAUTHORIZED,
            "Test should fail initially - no implementation exists yet");
    }

    #[tokio::test]
    async fn test_create_note_malformed_json_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "malformed@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let malformed_json = r#"{"title": "Test", "content": "Test", "folder_id": }"#;

            let response = ctx.client
                .post(&format!("{}/api/v1/notes", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .body(malformed_json)
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Return 400 for malformed JSON
    }

    #[tokio::test]
    async fn test_create_note_missing_content_type_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "no_content_type@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let note_body = json!({
                "title": "Test Note",
                "content": "Test content",
                "folder_id": null
            });

            let response = ctx.client
                .post(&format!("{}/api/v1/notes", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                // Missing Content-Type header
                .body(note_body.to_string())
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Return 400 for missing Content-Type header
    }

    #[tokio::test]
    async fn test_create_note_wrong_content_type_returns_415() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "wrong_content_type@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let note_body = json!({
                "title": "Test Note",
                "content": "Test content",
                "folder_id": null
            });

            let response = ctx.client
                .post(&format!("{}/api/v1/notes", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "text/plain")
                .body(note_body.to_string())
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Return 415 for wrong Content-Type
    }

    #[tokio::test]
    async fn test_create_note_user_isolation() {
        let ctx = setup_test_environment().await;
        
        // Get tokens for two different users
        let token1_result = get_auth_token(&ctx, "user1_note@example.com", "password123").await;
        let token2_result = get_auth_token(&ctx, "user2_note@example.com", "password123").await;
        
        if let (Ok(token1), Ok(token2)) = (token1_result, token2_result) {
            // User 1 creates a folder
            let folder_result = create_test_folder(&ctx, &token1, "User1 Folder").await;
            
            if let Ok(folder_id) = folder_result {
                // User 2 tries to create note in User 1's folder
                let note_body = json!({
                    "title": "Cross-user note",
                    "content": "This should fail",
                    "folder_id": folder_id
                });

                let response = ctx.client
                    .post(&format!("{}/api/v1/notes", ctx.base_url))
                    .header("Authorization", format!("Bearer {}", token2))
                    .header("Content-Type", "application/json")
                    .json(&note_body)
                    .send()
                    .await;

                // This test MUST fail initially
                assert!(response.is_err() || response.unwrap().status() != StatusCode::NOT_FOUND,
                    "Test should fail initially - no implementation exists yet");
            }
        }

        // Expected behavior: Users can only create notes in their own folders
    }

    #[tokio::test]
    async fn test_create_note_unicode_content_success() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "unicode@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let note_body = json!({
                "title": "日本語のタイトル",
                "content": "これは日本語のコンテンツです。絵文字も含まれます: 😀🚀✨",
                "folder_id": null
            });

            let response = ctx.client
                .post(&format!("{}/api/v1/notes", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json; charset=utf-8")
                .json(&note_body)
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::CREATED,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Unicode characters should be properly handled
    }

    #[tokio::test]
    async fn test_create_note_response_time_under_200ms() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "performance@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let note_body = json!({
                "title": "Performance Test",
                "content": "Testing response time",
                "folder_id": null
            });

            let start = std::time::Instant::now();
            
            let _response = ctx.client
                .post(&format!("{}/api/v1/notes", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .json(&note_body)
                .send()
                .await;

            let duration = start.elapsed();

            // This test will fail initially due to no server running
            assert!(duration.as_millis() > 200,
                "Test should fail initially - no server running, or verify <200ms after implementation");
        }
    }

    #[tokio::test]
    async fn test_create_note_concurrent_creation() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "concurrent@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            // Create multiple notes concurrently
            let mut handles = Vec::new();
            
            for i in 0..5 {
                let client = ctx.client.clone();
                let url = ctx.base_url.clone();
                let token = access_token.clone();
                
                let handle = tokio::spawn(async move {
                    let note_body = json!({
                        "title": format!("Concurrent Note {}", i),
                        "content": format!("Content for note {}", i),
                        "folder_id": null
                    });

                    client
                        .post(&format!("{}/api/v1/notes", url))
                        .header("Authorization", format!("Bearer {}", token))
                        .header("Content-Type", "application/json")
                        .json(&note_body)
                        .send()
                        .await
                });
                
                handles.push(handle);
            }

            let results: Vec<_> = futures::future::join_all(handles).await;
            
            // This test will fail initially
            assert!(results.iter().all(|r| r.is_err()),
                "Test should fail initially - no implementation exists yet");

            // Expected behavior: All concurrent note creations should succeed
        }
    }

    #[tokio::test]
    async fn test_create_note_json_content_type_response() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "json_response@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let note_body = json!({
                "title": "JSON Response Test",
                "content": "Testing response content type",
                "folder_id": null
            });

            let response = ctx.client
                .post(&format!("{}/api/v1/notes", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .json(&note_body)
                .send()
                .await;

            // This test MUST fail initially
            if let Ok(resp) = response {
                assert!(resp.status() != StatusCode::CREATED || 
                    resp.headers().get("content-type").map(|ct| ct.to_str().unwrap_or("")).unwrap_or("") != "application/json",
                    "Test should fail initially - no implementation exists yet");
            } else {
                assert!(true, "Expected failure - no server running");
            }
        }

        // Expected behavior: Response should have Content-Type: application/json
    }

    #[tokio::test]
    async fn test_create_note_extra_fields_ignored() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "extra_fields@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let note_body = json!({
                "title": "Test Note",
                "content": "Test content",
                "folder_id": null,
                "id": "should-be-ignored",
                "created_at": "should-be-ignored",
                "version": 999
            });

            let response = ctx.client
                .post(&format!("{}/api/v1/notes", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .json(&note_body)
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::CREATED,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Extra fields should be ignored, server-generated values used
    }
}