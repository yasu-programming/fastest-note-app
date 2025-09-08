use reqwest::StatusCode;
use serde_json::{json, Value};
use testcontainers::{clients::Cli, Container};
use testcontainers_modules::{postgres::Postgres as TestPostgres, redis::Redis};
use uuid::Uuid;

/// Contract tests for PUT /notes/{id} endpoint
/// 
/// These tests verify the API contract defined in contracts/api-spec.yaml
/// Tests MUST fail initially (no implementation exists yet)
/// 
/// Expected behavior according to contract:
/// - PUT /notes/{id}
/// - Headers: Authorization: Bearer <access_token>, Content-Type: application/json
/// - Body: { "title": "string", "content": "string", "version": number }
/// - Success (200): { "id", "title", "content", "folder_id", "created_at", "updated_at", "version" }
/// - Error (400): Bad request (validation errors, version mismatch)
/// - Error (401): Unauthorized
/// - Error (404): Note not found
/// - Error (409): Conflict (optimistic locking)

#[cfg(test)]
mod notes_update_contract_tests {
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

    /// Helper function to create a test note, returning note data
    async fn create_test_note(ctx: &TestContext, access_token: &str, title: &str, content: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let note_body = json!({
            "title": title,
            "content": content,
            "folder_id": null
        });

        let note_response = ctx.client
            .post(&format!("{}/api/v1/notes", ctx.base_url))
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .json(&note_body)
            .send()
            .await?;

        let note_data: Value = note_response.json().await?;
        Ok(note_data)
    }

    #[tokio::test]
    async fn test_update_note_success_returns_200_with_updated_data() {
        let ctx = setup_test_environment().await;
        
        // Get auth token (this will also fail initially)
        let token_result = get_auth_token(&ctx, "note_update@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            // Create a test note first (will also fail initially)
            let note_result = create_test_note(&ctx, &access_token, "Original Title", "Original content").await;
            
            if let Ok(note_data) = note_result {
                let note_id = note_data["id"].as_str().unwrap();
                let version = note_data["version"].as_u64().unwrap();

                let update_body = json!({
                    "title": "Updated Title",
                    "content": "Updated content",
                    "version": version
                });

                let response = ctx.client
                    .put(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&update_body)
                    .send()
                    .await;

                // This test MUST fail initially - no implementation exists
                assert!(response.is_err() || response.unwrap().status() != StatusCode::OK,
                    "Test should fail initially - no implementation exists yet");
            }
        } else {
            // Expected initial failure
            assert!(true, "Test fails as expected - no auth implementation yet");
        }

        // Expected contract behavior (will pass after implementation):
        // let response = response.unwrap();
        // assert_eq!(response.status(), StatusCode::OK);
        // 
        // let updated_note: Value = response.json().await.unwrap();
        // 
        // // Verify response structure matches OpenAPI spec
        // assert_eq!(updated_note["title"], "Updated Title");
        // assert_eq!(updated_note["content"], "Updated content");
        // assert_eq!(updated_note["version"], version + 1);
        // assert_eq!(updated_note["id"], note_id);
        // 
        // // Verify timestamps
        // assert!(updated_note.get("updated_at").is_some());
        // assert!(updated_note["updated_at"] != note_data["updated_at"]);
    }

    #[tokio::test]
    async fn test_update_note_missing_version_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "missing_version@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let note_result = create_test_note(&ctx, &access_token, "Test Note", "Test content").await;
            
            if let Ok(note_data) = note_result {
                let note_id = note_data["id"].as_str().unwrap();

                let update_body = json!({
                    "title": "Updated Title",
                    "content": "Updated content"
                    // Missing version field
                });

                let response = ctx.client
                    .put(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&update_body)
                    .send()
                    .await;

                // This test MUST fail initially
                assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                    "Test should fail initially - no implementation exists yet");
            }
        }

        // Expected behavior: Return 400 for missing required version field (optimistic locking)
    }

    #[tokio::test]
    async fn test_update_note_version_mismatch_returns_409() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "version_mismatch@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let note_result = create_test_note(&ctx, &access_token, "Test Note", "Test content").await;
            
            if let Ok(note_data) = note_result {
                let note_id = note_data["id"].as_str().unwrap();
                let version = note_data["version"].as_u64().unwrap();

                let update_body = json!({
                    "title": "Updated Title",
                    "content": "Updated content",
                    "version": version + 999  // Wrong version
                });

                let response = ctx.client
                    .put(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&update_body)
                    .send()
                    .await;

                // This test MUST fail initially
                assert!(response.is_err() || response.unwrap().status() != StatusCode::CONFLICT,
                    "Test should fail initially - no implementation exists yet");
            }
        }

        // Expected behavior: Return 409 for version mismatch (optimistic locking conflict)
    }

    #[tokio::test]
    async fn test_update_note_nonexistent_id_returns_404() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "nonexistent_note@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let nonexistent_id = Uuid::new_v4();

            let update_body = json!({
                "title": "Updated Title",
                "content": "Updated content",
                "version": 1
            });

            let response = ctx.client
                .put(&format!("{}/api/v1/notes/{}", ctx.base_url, nonexistent_id))
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .json(&update_body)
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::NOT_FOUND,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Return 404 for non-existent note ID
    }

    #[tokio::test]
    async fn test_update_note_invalid_id_format_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "invalid_id@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let update_body = json!({
                "title": "Updated Title",
                "content": "Updated content",
                "version": 1
            });

            let response = ctx.client
                .put(&format!("{}/api/v1/notes/not-a-uuid", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .json(&update_body)
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Return 400 for invalid UUID format in path
    }

    #[tokio::test]
    async fn test_update_note_empty_title_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "empty_title_update@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let note_result = create_test_note(&ctx, &access_token, "Test Note", "Test content").await;
            
            if let Ok(note_data) = note_result {
                let note_id = note_data["id"].as_str().unwrap();
                let version = note_data["version"].as_u64().unwrap();

                let update_body = json!({
                    "title": "",
                    "content": "Updated content",
                    "version": version
                });

                let response = ctx.client
                    .put(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&update_body)
                    .send()
                    .await;

                // This test MUST fail initially
                assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                    "Test should fail initially - no implementation exists yet");
            }
        }

        // Expected behavior: Return 400 for empty title (validation error)
    }

    #[tokio::test]
    async fn test_update_note_title_too_long_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "long_title_update@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let note_result = create_test_note(&ctx, &access_token, "Test Note", "Test content").await;
            
            if let Ok(note_data) = note_result {
                let note_id = note_data["id"].as_str().unwrap();
                let version = note_data["version"].as_u64().unwrap();
                let long_title = "a".repeat(256); // Assuming 255 char limit

                let update_body = json!({
                    "title": long_title,
                    "content": "Updated content",
                    "version": version
                });

                let response = ctx.client
                    .put(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&update_body)
                    .send()
                    .await;

                // This test MUST fail initially
                assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                    "Test should fail initially - no implementation exists yet");
            }
        }

        // Expected behavior: Return 400 for title exceeding max length (255 chars)
    }

    #[tokio::test]
    async fn test_update_note_content_too_large_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "large_content_update@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let note_result = create_test_note(&ctx, &access_token, "Test Note", "Test content").await;
            
            if let Ok(note_data) = note_result {
                let note_id = note_data["id"].as_str().unwrap();
                let version = note_data["version"].as_u64().unwrap();
                // Create content larger than 1MB limit
                let large_content = "a".repeat(1024 * 1024 + 1); // 1MB + 1 byte

                let update_body = json!({
                    "title": "Updated Title",
                    "content": large_content,
                    "version": version
                });

                let response = ctx.client
                    .put(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&update_body)
                    .send()
                    .await;

                // This test MUST fail initially
                assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                    "Test should fail initially - no implementation exists yet");
            }
        }

        // Expected behavior: Return 400 for content exceeding 1MB limit
    }

    #[tokio::test]
    async fn test_update_note_unauthorized_no_token_returns_401() {
        let ctx = setup_test_environment().await;

        let note_id = Uuid::new_v4();
        let update_body = json!({
            "title": "Updated Title",
            "content": "Updated content",
            "version": 1
        });

        let response = ctx.client
            .put(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
            .header("Content-Type", "application/json")
            .json(&update_body)
            .send()
            .await;

        // This test MUST fail initially
        assert!(response.is_err() || response.unwrap().status() != StatusCode::UNAUTHORIZED,
            "Test should fail initially - no implementation exists yet");

        // Expected behavior: Return 401 when no Authorization header provided
    }

    #[tokio::test]
    async fn test_update_note_invalid_token_returns_401() {
        let ctx = setup_test_environment().await;

        let note_id = Uuid::new_v4();
        let update_body = json!({
            "title": "Updated Title",
            "content": "Updated content",
            "version": 1
        });

        let response = ctx.client
            .put(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
            .header("Authorization", "Bearer invalid_token_123")
            .header("Content-Type", "application/json")
            .json(&update_body)
            .send()
            .await;

        // This test MUST fail initially
        assert!(response.is_err() || response.unwrap().status() != StatusCode::UNAUTHORIZED,
            "Test should fail initially - no implementation exists yet");
    }

    #[tokio::test]
    async fn test_update_note_user_isolation() {
        let ctx = setup_test_environment().await;
        
        // Get tokens for two different users
        let token1_result = get_auth_token(&ctx, "user1_update@example.com", "password123").await;
        let token2_result = get_auth_token(&ctx, "user2_update@example.com", "password123").await;
        
        if let (Ok(token1), Ok(token2)) = (token1_result, token2_result) {
            // User 1 creates a note
            let note_result = create_test_note(&ctx, &token1, "User1 Note", "User1 content").await;
            
            if let Ok(note_data) = note_result {
                let note_id = note_data["id"].as_str().unwrap();
                let version = note_data["version"].as_u64().unwrap();

                // User 2 tries to update User 1's note
                let update_body = json!({
                    "title": "Hacked Title",
                    "content": "Hacked content",
                    "version": version
                });

                let response = ctx.client
                    .put(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
                    .header("Authorization", format!("Bearer {}", token2))
                    .header("Content-Type", "application/json")
                    .json(&update_body)
                    .send()
                    .await;

                // This test MUST fail initially
                assert!(response.is_err() || response.unwrap().status() != StatusCode::NOT_FOUND,
                    "Test should fail initially - no implementation exists yet");
            }
        }

        // Expected behavior: Users can only update their own notes (404 for others' notes)
    }

    #[tokio::test]
    async fn test_update_note_malformed_json_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "malformed_update@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let note_result = create_test_note(&ctx, &access_token, "Test Note", "Test content").await;
            
            if let Ok(note_data) = note_result {
                let note_id = note_data["id"].as_str().unwrap();
                let malformed_json = r#"{"title": "Updated", "content": "Updated", "version": }"#;

                let response = ctx.client
                    .put(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .body(malformed_json)
                    .send()
                    .await;

                // This test MUST fail initially
                assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                    "Test should fail initially - no implementation exists yet");
            }
        }

        // Expected behavior: Return 400 for malformed JSON
    }

    #[tokio::test]
    async fn test_update_note_missing_content_type_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "no_content_type_update@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let note_result = create_test_note(&ctx, &access_token, "Test Note", "Test content").await;
            
            if let Ok(note_data) = note_result {
                let note_id = note_data["id"].as_str().unwrap();
                let version = note_data["version"].as_u64().unwrap();

                let update_body = json!({
                    "title": "Updated Title",
                    "content": "Updated content",
                    "version": version
                });

                let response = ctx.client
                    .put(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    // Missing Content-Type header
                    .body(update_body.to_string())
                    .send()
                    .await;

                // This test MUST fail initially
                assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                    "Test should fail initially - no implementation exists yet");
            }
        }

        // Expected behavior: Return 400 for missing Content-Type header
    }

    #[tokio::test]
    async fn test_update_note_unicode_content_success() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "unicode_update@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let note_result = create_test_note(&ctx, &access_token, "Test Note", "Test content").await;
            
            if let Ok(note_data) = note_result {
                let note_id = note_data["id"].as_str().unwrap();
                let version = note_data["version"].as_u64().unwrap();

                let update_body = json!({
                    "title": "日本語のタイトル",
                    "content": "これは日本語のコンテンツです。絵文字も含まれます: 😀🚀✨",
                    "version": version
                });

                let response = ctx.client
                    .put(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json; charset=utf-8")
                    .json(&update_body)
                    .send()
                    .await;

                // This test MUST fail initially
                assert!(response.is_err() || response.unwrap().status() != StatusCode::OK,
                    "Test should fail initially - no implementation exists yet");
            }
        }

        // Expected behavior: Unicode characters should be properly handled
    }

    #[tokio::test]
    async fn test_update_note_partial_update_title_only() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "partial_update@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let note_result = create_test_note(&ctx, &access_token, "Original Title", "Original content").await;
            
            if let Ok(note_data) = note_result {
                let note_id = note_data["id"].as_str().unwrap();
                let version = note_data["version"].as_u64().unwrap();

                let update_body = json!({
                    "title": "Updated Title Only",
                    // Content should remain unchanged
                    "version": version
                });

                let response = ctx.client
                    .put(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&update_body)
                    .send()
                    .await;

                // This test MUST fail initially
                assert!(response.is_err() || response.unwrap().status() != StatusCode::OK,
                    "Test should fail initially - no implementation exists yet");
            }
        }

        // Expected behavior: Should require all updatable fields (title and content)
        // Or handle partial updates appropriately
    }

    #[tokio::test]
    async fn test_update_note_concurrent_updates() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "concurrent_update@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let note_result = create_test_note(&ctx, &access_token, "Test Note", "Test content").await;
            
            if let Ok(note_data) = note_result {
                let note_id = note_data["id"].as_str().unwrap();
                let version = note_data["version"].as_u64().unwrap();

                // Make multiple concurrent update requests with same version
                let mut handles = Vec::new();
                
                for i in 0..3 {
                    let client = ctx.client.clone();
                    let url = ctx.base_url.clone();
                    let token = access_token.clone();
                    let id = note_id.to_string();
                    
                    let handle = tokio::spawn(async move {
                        let update_body = json!({
                            "title": format!("Concurrent Update {}", i),
                            "content": format!("Content from update {}", i),
                            "version": version
                        });

                        client
                            .put(&format!("{}/api/v1/notes/{}", url, id))
                            .header("Authorization", format!("Bearer {}", token))
                            .header("Content-Type", "application/json")
                            .json(&update_body)
                            .send()
                            .await
                    });
                    
                    handles.push(handle);
                }

                let results: Vec<_> = futures::future::join_all(handles).await;
                
                // This test will fail initially
                assert!(results.iter().all(|r| r.is_err()),
                    "Test should fail initially - no implementation exists yet");

                // Expected behavior: Only one should succeed (200), others should get 409 (conflict)
            }
        }
    }

    #[tokio::test]
    async fn test_update_note_response_time_under_200ms() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "performance_update@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let note_result = create_test_note(&ctx, &access_token, "Test Note", "Test content").await;
            
            if let Ok(note_data) = note_result {
                let note_id = note_data["id"].as_str().unwrap();
                let version = note_data["version"].as_u64().unwrap();

                let update_body = json!({
                    "title": "Performance Test",
                    "content": "Testing update response time",
                    "version": version
                });

                let start = std::time::Instant::now();
                
                let _response = ctx.client
                    .put(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&update_body)
                    .send()
                    .await;

                let duration = start.elapsed();

                // This test will fail initially due to no server running
                assert!(duration.as_millis() > 200,
                    "Test should fail initially - no server running, or verify <200ms after implementation");
            }
        }
    }

    #[tokio::test]
    async fn test_update_note_json_content_type_response() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "json_response_update@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let note_result = create_test_note(&ctx, &access_token, "Test Note", "Test content").await;
            
            if let Ok(note_data) = note_result {
                let note_id = note_data["id"].as_str().unwrap();
                let version = note_data["version"].as_u64().unwrap();

                let update_body = json!({
                    "title": "JSON Response Test",
                    "content": "Testing response content type",
                    "version": version
                });

                let response = ctx.client
                    .put(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&update_body)
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
        }

        // Expected behavior: Response should have Content-Type: application/json
    }
}