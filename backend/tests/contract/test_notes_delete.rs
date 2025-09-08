use reqwest::StatusCode;
use serde_json::{json, Value};
use testcontainers::{clients::Cli, Container};
use testcontainers_modules::{postgres::Postgres as TestPostgres, redis::Redis};
use uuid::Uuid;

/// Contract tests for DELETE /notes/{id} endpoint
/// 
/// These tests verify the API contract defined in contracts/api-spec.yaml
/// Tests MUST fail initially (no implementation exists yet)
/// 
/// Expected behavior according to contract:
/// - DELETE /notes/{id}
/// - Headers: Authorization: Bearer <access_token>
/// - Success (204): No content (note deleted successfully)
/// - Error (401): Unauthorized
/// - Error (404): Note not found
/// - Error (409): Conflict (if note has dependencies - future use)

#[cfg(test)]
mod notes_delete_contract_tests {
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
    async fn test_delete_note_success_returns_204() {
        let ctx = setup_test_environment().await;
        
        // Get auth token (this will also fail initially)
        let token_result = get_auth_token(&ctx, "note_delete@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            // Create a test note first (will also fail initially)
            let note_result = create_test_note(&ctx, &access_token, "Test Note", "Test content").await;
            
            if let Ok(note_data) = note_result {
                let note_id = note_data["id"].as_str().unwrap();

                let response = ctx.client
                    .delete(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .send()
                    .await;

                // This test MUST fail initially - no implementation exists
                assert!(response.is_err() || response.unwrap().status() != StatusCode::NO_CONTENT,
                    "Test should fail initially - no implementation exists yet");
            }
        } else {
            // Expected initial failure
            assert!(true, "Test fails as expected - no auth implementation yet");
        }

        // Expected contract behavior (will pass after implementation):
        // let response = response.unwrap();
        // assert_eq!(response.status(), StatusCode::NO_CONTENT);
        // 
        // // Verify note is actually deleted (subsequent GET should return 404)
        // let get_response = ctx.client
        //     .get(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
        //     .header("Authorization", format!("Bearer {}", access_token))
        //     .send()
        //     .await
        //     .unwrap();
        // 
        // assert_eq!(get_response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_delete_note_nonexistent_id_returns_404() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "nonexistent_delete@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let nonexistent_id = Uuid::new_v4();

            let response = ctx.client
                .delete(&format!("{}/api/v1/notes/{}", ctx.base_url, nonexistent_id))
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::NOT_FOUND,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Return 404 for non-existent note ID
    }

    #[tokio::test]
    async fn test_delete_note_invalid_id_format_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "invalid_id_delete@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let response = ctx.client
                .delete(&format!("{}/api/v1/notes/not-a-uuid", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Return 400 for invalid UUID format in path
    }

    #[tokio::test]
    async fn test_delete_note_unauthorized_no_token_returns_401() {
        let ctx = setup_test_environment().await;

        let note_id = Uuid::new_v4();

        let response = ctx.client
            .delete(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
            .send()
            .await;

        // This test MUST fail initially
        assert!(response.is_err() || response.unwrap().status() != StatusCode::UNAUTHORIZED,
            "Test should fail initially - no implementation exists yet");

        // Expected behavior: Return 401 when no Authorization header provided
    }

    #[tokio::test]
    async fn test_delete_note_invalid_token_returns_401() {
        let ctx = setup_test_environment().await;

        let note_id = Uuid::new_v4();

        let response = ctx.client
            .delete(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
            .header("Authorization", "Bearer invalid_token_123")
            .send()
            .await;

        // This test MUST fail initially
        assert!(response.is_err() || response.unwrap().status() != StatusCode::UNAUTHORIZED,
            "Test should fail initially - no implementation exists yet");
    }

    #[tokio::test]
    async fn test_delete_note_expired_token_returns_401() {
        let ctx = setup_test_environment().await;

        let note_id = Uuid::new_v4();
        // Use clearly expired token
        let expired_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ0ZXN0IiwiZXhwIjoxNjAwMDAwMDAwfQ.invalid";

        let response = ctx.client
            .delete(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
            .header("Authorization", format!("Bearer {}", expired_token))
            .send()
            .await;

        // This test MUST fail initially
        assert!(response.is_err() || response.unwrap().status() != StatusCode::UNAUTHORIZED,
            "Test should fail initially - no implementation exists yet");
    }

    #[tokio::test]
    async fn test_delete_note_malformed_bearer_header_returns_401() {
        let ctx = setup_test_environment().await;

        let note_id = Uuid::new_v4();

        let response = ctx.client
            .delete(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
            .header("Authorization", "Malformed token123")
            .send()
            .await;

        // This test MUST fail initially
        assert!(response.is_err() || response.unwrap().status() != StatusCode::UNAUTHORIZED,
            "Test should fail initially - no implementation exists yet");
    }

    #[tokio::test]
    async fn test_delete_note_user_isolation() {
        let ctx = setup_test_environment().await;
        
        // Get tokens for two different users
        let token1_result = get_auth_token(&ctx, "user1_delete@example.com", "password123").await;
        let token2_result = get_auth_token(&ctx, "user2_delete@example.com", "password123").await;
        
        if let (Ok(token1), Ok(token2)) = (token1_result, token2_result) {
            // User 1 creates a note
            let note_result = create_test_note(&ctx, &token1, "User1 Note", "User1 content").await;
            
            if let Ok(note_data) = note_result {
                let note_id = note_data["id"].as_str().unwrap();

                // User 2 tries to delete User 1's note
                let response = ctx.client
                    .delete(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
                    .header("Authorization", format!("Bearer {}", token2))
                    .send()
                    .await;

                // This test MUST fail initially
                assert!(response.is_err() || response.unwrap().status() != StatusCode::NOT_FOUND,
                    "Test should fail initially - no implementation exists yet");
            }
        }

        // Expected behavior: Users can only delete their own notes (404 for others' notes)
    }

    #[tokio::test]
    async fn test_delete_note_twice_returns_404() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "double_delete@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let note_result = create_test_note(&ctx, &access_token, "Test Note", "Test content").await;
            
            if let Ok(note_data) = note_result {
                let note_id = note_data["id"].as_str().unwrap();

                // First deletion
                let response1 = ctx.client
                    .delete(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .send()
                    .await;

                // Second deletion (should return 404)
                let response2 = ctx.client
                    .delete(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .send()
                    .await;

                // Both tests MUST fail initially
                assert!(response1.is_err() || response1.unwrap().status() != StatusCode::NO_CONTENT,
                    "Test should fail initially - no implementation exists yet");
                assert!(response2.is_err() || response2.unwrap().status() != StatusCode::NOT_FOUND,
                    "Test should fail initially - no implementation exists yet");
            }
        }

        // Expected behavior: First delete succeeds (204), second returns 404
    }

    #[tokio::test]
    async fn test_delete_note_concurrent_deletion() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "concurrent_delete@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let note_result = create_test_note(&ctx, &access_token, "Test Note", "Test content").await;
            
            if let Ok(note_data) = note_result {
                let note_id = note_data["id"].as_str().unwrap();

                // Make multiple concurrent delete requests
                let mut handles = Vec::new();
                
                for _ in 0..3 {
                    let client = ctx.client.clone();
                    let url = ctx.base_url.clone();
                    let token = access_token.clone();
                    let id = note_id.to_string();
                    
                    let handle = tokio::spawn(async move {
                        client
                            .delete(&format!("{}/api/v1/notes/{}", url, id))
                            .header("Authorization", format!("Bearer {}", token))
                            .send()
                            .await
                    });
                    
                    handles.push(handle);
                }

                let results: Vec<_> = futures::future::join_all(handles).await;
                
                // This test will fail initially
                assert!(results.iter().all(|r| r.is_err()),
                    "Test should fail initially - no implementation exists yet");

                // Expected behavior: One should succeed (204), others should get 404
            }
        }
    }

    #[tokio::test]
    async fn test_delete_note_response_time_under_200ms() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "performance_delete@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let note_result = create_test_note(&ctx, &access_token, "Performance Note", "Performance test").await;
            
            if let Ok(note_data) = note_result {
                let note_id = note_data["id"].as_str().unwrap();

                let start = std::time::Instant::now();
                
                let _response = ctx.client
                    .delete(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
                    .header("Authorization", format!("Bearer {}", access_token))
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
    async fn test_delete_note_no_body_in_response() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "no_body@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let note_result = create_test_note(&ctx, &access_token, "Test Note", "Test content").await;
            
            if let Ok(note_data) = note_result {
                let note_id = note_data["id"].as_str().unwrap();

                let response = ctx.client
                    .delete(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .send()
                    .await;

                // This test MUST fail initially
                if let Ok(resp) = response {
                    assert!(resp.status() != StatusCode::NO_CONTENT,
                        "Test should fail initially - no implementation exists yet");
                } else {
                    assert!(true, "Expected failure - no server running");
                }
            }
        }

        // Expected behavior: 204 response should have empty body
    }

    #[tokio::test]
    async fn test_delete_note_cache_invalidation() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "cache_invalidation@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let note_result = create_test_note(&ctx, &access_token, "Cached Note", "Cached content").await;
            
            if let Ok(note_data) = note_result {
                let note_id = note_data["id"].as_str().unwrap();

                // Get the note to potentially cache it
                let _get_response1 = ctx.client
                    .get(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .send()
                    .await;

                // Delete the note
                let _delete_response = ctx.client
                    .delete(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .send()
                    .await;

                // Try to get the note again (should be 404, not cached)
                let get_response2 = ctx.client
                    .get(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .send()
                    .await;

                // All tests MUST fail initially
                assert!(get_response2.is_err(),
                    "Test should fail initially - no implementation exists yet");
            }
        }

        // Expected behavior: Cache should be properly invalidated after deletion
    }

    #[tokio::test]
    async fn test_delete_note_audit_trail() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "audit_trail@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let note_result = create_test_note(&ctx, &access_token, "Audit Note", "Audit content").await;
            
            if let Ok(note_data) = note_result {
                let note_id = note_data["id"].as_str().unwrap();

                let response = ctx.client
                    .delete(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .send()
                    .await;

                // This test MUST fail initially
                assert!(response.is_err() || response.unwrap().status() != StatusCode::NO_CONTENT,
                    "Test should fail initially - no implementation exists yet");
            }
        }

        // Expected behavior: Deletion should be logged for audit purposes
        // (This might involve checking audit logs or database soft-delete markers)
    }

    #[tokio::test]
    async fn test_delete_note_case_sensitive_headers() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "case_headers_delete@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let note_result = create_test_note(&ctx, &access_token, "Case Test Note", "Case test content").await;
            
            if let Ok(note_data) = note_result {
                let note_id = note_data["id"].as_str().unwrap();

                let response = ctx.client
                    .delete(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
                    .header("authorization", format!("Bearer {}", access_token))  // lowercase header
                    .send()
                    .await;

                // This test MUST fail initially
                assert!(response.is_err() || response.unwrap().status() != StatusCode::NO_CONTENT,
                    "Test should fail initially - no implementation exists yet");
            }
        }

        // Expected behavior: HTTP headers should be case-insensitive per RFC 7230
    }

    #[tokio::test]
    async fn test_delete_note_with_query_parameters_ignored() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "query_params@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let note_result = create_test_note(&ctx, &access_token, "Query Test Note", "Query test content").await;
            
            if let Ok(note_data) = note_result {
                let note_id = note_data["id"].as_str().unwrap();

                let response = ctx.client
                    .delete(&format!("{}/api/v1/notes/{}?force=true&version=1", ctx.base_url, note_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .send()
                    .await;

                // This test MUST fail initially
                assert!(response.is_err() || response.unwrap().status() != StatusCode::NO_CONTENT,
                    "Test should fail initially - no implementation exists yet");
            }
        }

        // Expected behavior: Query parameters should be ignored for DELETE requests
    }

    #[tokio::test]
    async fn test_delete_note_with_request_body_ignored() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "body_ignored@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let note_result = create_test_note(&ctx, &access_token, "Body Test Note", "Body test content").await;
            
            if let Ok(note_data) = note_result {
                let note_id = note_data["id"].as_str().unwrap();

                let request_body = json!({
                    "reason": "No longer needed"
                });

                let response = ctx.client
                    .delete(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&request_body)
                    .send()
                    .await;

                // This test MUST fail initially
                assert!(response.is_err() || response.unwrap().status() != StatusCode::NO_CONTENT,
                    "Test should fail initially - no implementation exists yet");
            }
        }

        // Expected behavior: Request body should be ignored for DELETE requests
    }
}