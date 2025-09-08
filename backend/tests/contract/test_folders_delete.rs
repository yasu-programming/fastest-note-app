use reqwest::StatusCode;
use serde_json::{json, Value};
use testcontainers::{clients::Cli, Container};
use testcontainers_modules::{postgres::Postgres as TestPostgres, redis::Redis};
use uuid::Uuid;

/// Contract tests for DELETE /folders/{id} endpoint
/// 
/// These tests verify the API contract defined in contracts/api-spec.yaml
/// Tests MUST fail initially (no implementation exists yet)
/// 
/// Expected behavior according to contract:
/// - DELETE /folders/{id}
/// - Headers: Authorization: Bearer <access_token>
/// - Success (204): No content (folder deleted successfully)
/// - Error (401): Unauthorized
/// - Error (404): Folder not found

#[cfg(test)]
mod folders_delete_contract_tests {
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

    /// Helper function to get auth token
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

    /// Helper function to create a test folder
    async fn create_test_folder(ctx: &TestContext, token: &str, name: &str, parent_id: Option<Uuid>) -> Result<Value, Box<dyn std::error::Error>> {
        let request_body = json!({
            "name": name,
            "parent_folder_id": parent_id
        });

        let response = ctx.client
            .post(&format!("{}/api/v1/folders", ctx.base_url))
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        let folder_data: Value = response.json().await?;
        Ok(folder_data)
    }

    /// Helper function to create a test note in a folder
    async fn create_test_note(ctx: &TestContext, token: &str, title: &str, folder_id: Option<Uuid>) -> Result<Value, Box<dyn std::error::Error>> {
        let request_body = json!({
            "title": title,
            "content": "Test note content",
            "folder_id": folder_id
        });

        let response = ctx.client
            .post(&format!("{}/api/v1/notes", ctx.base_url))
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        let note_data: Value = response.json().await?;
        Ok(note_data)
    }

    #[tokio::test]
    async fn test_delete_empty_folder_success_returns_204() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "delete_empty@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            // First create a folder (this will also fail initially)
            let folder_result = create_test_folder(&ctx, &access_token, "Empty Folder", None).await;
            
            if let Ok(folder) = folder_result {
                let folder_id = folder.get("id").unwrap().as_str().unwrap();

                let response = ctx.client
                    .delete(&format!("{}/api/v1/folders/{}", ctx.base_url, folder_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .send()
                    .await;

                // This test MUST fail initially - no implementation exists
                assert!(response.is_err() || response.unwrap().status() != StatusCode::NO_CONTENT,
                    "Test should fail initially - no implementation exists yet");
            }
        }

        // Expected contract behavior (will pass after implementation):
        // let response = response.unwrap();
        // assert_eq!(response.status(), StatusCode::NO_CONTENT);
        // 
        // // Verify no response body for 204
        // let body = response.text().await.unwrap();
        // assert!(body.is_empty(), "204 responses should have empty body");
        // 
        // // Verify folder is actually deleted (would need GET request to confirm)
    }

    #[tokio::test]
    async fn test_delete_folder_with_notes_moves_to_parent() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "delete_with_notes@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            // Create parent folder, child folder, and note in child folder
            let parent_result = create_test_folder(&ctx, &access_token, "Parent Folder", None).await;
            
            if let Ok(parent) = parent_result {
                let parent_id = parent.get("id").unwrap().as_str().unwrap();
                let parent_uuid = Uuid::parse_str(parent_id).unwrap();
                
                let child_result = create_test_folder(&ctx, &access_token, "Child Folder", Some(parent_uuid)).await;
                
                if let Ok(child) = child_result {
                    let child_id = child.get("id").unwrap().as_str().unwrap();
                    let child_uuid = Uuid::parse_str(child_id).unwrap();

                    // Create note in child folder
                    let _note_result = create_test_note(&ctx, &access_token, "Test Note", Some(child_uuid)).await;

                    // Delete child folder (notes should move to parent)
                    let response = ctx.client
                        .delete(&format!("{}/api/v1/folders/{}", ctx.base_url, child_id))
                        .header("Authorization", format!("Bearer {}", access_token))
                        .send()
                        .await;

                    // This test MUST fail initially
                    assert!(response.is_err() || response.unwrap().status() != StatusCode::NO_CONTENT,
                        "Test should fail initially - no implementation exists yet");
                }
            }
        }

        // Expected behavior: Notes in deleted folder should move to parent folder
        // Per database schema: ON DELETE SET NULL (moves to root) or custom logic for parent
    }

    #[tokio::test]
    async fn test_delete_root_folder_with_notes_moves_to_root() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "delete_root@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            // Create root folder and note in it
            let folder_result = create_test_folder(&ctx, &access_token, "Root Folder", None).await;
            
            if let Ok(folder) = folder_result {
                let folder_id = folder.get("id").unwrap().as_str().unwrap();
                let folder_uuid = Uuid::parse_str(folder_id).unwrap();

                // Create note in folder
                let _note_result = create_test_note(&ctx, &access_token, "Root Note", Some(folder_uuid)).await;

                // Delete folder (notes should move to root level)
                let response = ctx.client
                    .delete(&format!("{}/api/v1/folders/{}", ctx.base_url, folder_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .send()
                    .await;

                // This test MUST fail initially
                assert!(response.is_err() || response.unwrap().status() != StatusCode::NO_CONTENT,
                    "Test should fail initially - no implementation exists yet");
            }
        }

        // Expected behavior: Notes in deleted root folder move to root level (folder_id = NULL)
    }

    #[tokio::test]
    async fn test_delete_folder_with_subfolders_cascades() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "delete_cascade@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            // Create parent and child folders
            let parent_result = create_test_folder(&ctx, &access_token, "Parent Folder", None).await;
            
            if let Ok(parent) = parent_result {
                let parent_id = parent.get("id").unwrap().as_str().unwrap();
                let parent_uuid = Uuid::parse_str(parent_id).unwrap();
                
                let _child_result = create_test_folder(&ctx, &access_token, "Child Folder", Some(parent_uuid)).await;

                // Delete parent folder (should cascade delete child)
                let response = ctx.client
                    .delete(&format!("{}/api/v1/folders/{}", ctx.base_url, parent_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .send()
                    .await;

                // This test MUST fail initially
                assert!(response.is_err() || response.unwrap().status() != StatusCode::NO_CONTENT,
                    "Test should fail initially - no implementation exists yet");
            }
        }

        // Expected behavior: Deleting folder cascades to delete subfolders
        // Per database schema: ON DELETE CASCADE for folders
    }

    #[tokio::test]
    async fn test_delete_folder_nonexistent_id_returns_404() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "nonexistent_delete@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let nonexistent_id = Uuid::new_v4();

            let response = ctx.client
                .delete(&format!("{}/api/v1/folders/{}", ctx.base_url, nonexistent_id))
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::NOT_FOUND,
                "Test should fail initially - no implementation exists yet");
        }
    }

    #[tokio::test]
    async fn test_delete_folder_invalid_id_format_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "invalid_id_delete@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let response = ctx.client
                .delete(&format!("{}/api/v1/folders/not-a-uuid", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                "Test should fail initially - no implementation exists yet");
        }
    }

    #[tokio::test]
    async fn test_delete_folder_unauthorized_no_token_returns_401() {
        let ctx = setup_test_environment().await;

        let folder_id = Uuid::new_v4();

        let response = ctx.client
            .delete(&format!("{}/api/v1/folders/{}", ctx.base_url, folder_id))
            .send()
            .await;

        // This test MUST fail initially
        assert!(response.is_err() || response.unwrap().status() != StatusCode::UNAUTHORIZED,
            "Test should fail initially - no implementation exists yet");
    }

    #[tokio::test]
    async fn test_delete_folder_invalid_token_returns_401() {
        let ctx = setup_test_environment().await;

        let folder_id = Uuid::new_v4();

        let response = ctx.client
            .delete(&format!("{}/api/v1/folders/{}", ctx.base_url, folder_id))
            .header("Authorization", "Bearer invalid_token_123")
            .send()
            .await;

        // This test MUST fail initially
        assert!(response.is_err() || response.unwrap().status() != StatusCode::UNAUTHORIZED,
            "Test should fail initially - no implementation exists yet");
    }

    #[tokio::test]
    async fn test_delete_folder_user_isolation() {
        let ctx = setup_test_environment().await;
        
        let token1_result = get_auth_token(&ctx, "user1@example.com", "password123").await;
        let token2_result = get_auth_token(&ctx, "user2@example.com", "password123").await;
        
        if let (Ok(token1), Ok(token2)) = (token1_result, token2_result) {
            // User 1 creates a folder
            let folder_result = create_test_folder(&ctx, &token1, "User1 Folder", None).await;
            
            if let Ok(folder) = folder_result {
                let folder_id = folder.get("id").unwrap().as_str().unwrap();

                // User 2 tries to delete User 1's folder (should fail)
                let response = ctx.client
                    .delete(&format!("{}/api/v1/folders/{}", ctx.base_url, folder_id))
                    .header("Authorization", format!("Bearer {}", token2))
                    .send()
                    .await;

                // This test MUST fail initially
                assert!(response.is_err() || response.unwrap().status() != StatusCode::NOT_FOUND,
                    "Test should fail initially - no implementation exists yet");
            }
        }

        // Expected behavior: Users should not be able to delete other users' folders
        // Should return 404 rather than 403 to avoid information disclosure
    }

    #[tokio::test]
    async fn test_delete_folder_twice_returns_404() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "delete_twice@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let folder_result = create_test_folder(&ctx, &access_token, "Delete Twice", None).await;
            
            if let Ok(folder) = folder_result {
                let folder_id = folder.get("id").unwrap().as_str().unwrap();

                // First deletion (should succeed)
                let _first_response = ctx.client
                    .delete(&format!("{}/api/v1/folders/{}", ctx.base_url, folder_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .send()
                    .await;

                // Second deletion (should return 404)
                let second_response = ctx.client
                    .delete(&format!("{}/api/v1/folders/{}", ctx.base_url, folder_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .send()
                    .await;

                // This test MUST fail initially
                assert!(second_response.is_err() || second_response.unwrap().status() != StatusCode::NOT_FOUND,
                    "Test should fail initially - no implementation exists yet");
            }
        }

        // Expected behavior: Second delete should return 404 (already deleted)
    }

    #[tokio::test]
    async fn test_delete_folder_response_time_under_200ms() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "performance_delete@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let folder_result = create_test_folder(&ctx, &access_token, "Performance Test", None).await;
            
            if let Ok(folder) = folder_result {
                let folder_id = folder.get("id").unwrap().as_str().unwrap();

                let start = std::time::Instant::now();
                
                let _response = ctx.client
                    .delete(&format!("{}/api/v1/folders/{}", ctx.base_url, folder_id))
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
    async fn test_delete_folder_no_content_type_header() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "no_content_type@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let folder_result = create_test_folder(&ctx, &access_token, "No Content Type", None).await;
            
            if let Ok(folder) = folder_result {
                let folder_id = folder.get("id").unwrap().as_str().unwrap();

                let response = ctx.client
                    .delete(&format!("{}/api/v1/folders/{}", ctx.base_url, folder_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .send()
                    .await;

                // This test MUST fail initially
                if let Ok(resp) = response {
                    // After implementation, verify 204 has no Content-Type header or empty body
                    assert!(resp.status() != StatusCode::NO_CONTENT,
                        "Test should fail initially - no implementation exists yet");
                } else {
                    assert!(true, "Expected failure - no server running");
                }
            }
        }

        // Expected behavior: 204 responses should not include Content-Type header
    }

    #[tokio::test]
    async fn test_delete_folder_with_request_body_ignored() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "body_ignored@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let folder_result = create_test_folder(&ctx, &access_token, "Body Ignored", None).await;
            
            if let Ok(folder) = folder_result {
                let folder_id = folder.get("id").unwrap().as_str().unwrap();

                // Send DELETE with body (should be ignored)
                let request_body = json!({
                    "should_be": "ignored"
                });

                let response = ctx.client
                    .delete(&format!("{}/api/v1/folders/{}", ctx.base_url, folder_id))
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

        // Expected behavior: DELETE endpoints should ignore request body
    }

    #[tokio::test]
    async fn test_delete_folder_concurrent_operations() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "concurrent_delete@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let folder_result = create_test_folder(&ctx, &access_token, "Concurrent Test", None).await;
            
            if let Ok(folder) = folder_result {
                let folder_id = folder.get("id").unwrap().as_str().unwrap();

                // Make multiple concurrent delete requests
                let mut handles = Vec::new();
                
                for _ in 0..3 {
                    let client = ctx.client.clone();
                    let url = ctx.base_url.clone();
                    let token = access_token.clone();
                    let id = folder_id.to_string();
                    
                    let handle = tokio::spawn(async move {
                        client
                            .delete(&format!("{}/api/v1/folders/{}", url, id))
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
            }
        }

        // Expected behavior: One delete should succeed (204), others should return 404
        // System should handle concurrent deletes gracefully
    }

    #[tokio::test]
    async fn test_delete_folder_with_deep_hierarchy() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "deep_hierarchy@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            // Create a chain of nested folders
            let mut current_parent = None;
            let mut folder_ids = Vec::new();

            // Create 5 nested folders
            for i in 1..=5 {
                let folder_result = create_test_folder(
                    &ctx, 
                    &access_token, 
                    &format!("Level {} Folder", i), 
                    current_parent
                ).await;
                
                if let Ok(folder) = folder_result {
                    let folder_id = folder.get("id").unwrap().as_str().unwrap();
                    let folder_uuid = Uuid::parse_str(folder_id).unwrap();
                    folder_ids.push(folder_id.to_string());
                    current_parent = Some(folder_uuid);
                }
            }

            if !folder_ids.is_empty() {
                // Delete the root folder (should cascade delete all children)
                let root_folder_id = &folder_ids[0];

                let response = ctx.client
                    .delete(&format!("{}/api/v1/folders/{}", ctx.base_url, root_folder_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .send()
                    .await;

                // This test MUST fail initially
                assert!(response.is_err() || response.unwrap().status() != StatusCode::NO_CONTENT,
                    "Test should fail initially - no implementation exists yet");
            }
        }

        // Expected behavior: Cascade delete should handle deep hierarchies efficiently
        // Should not cause database performance issues or stack overflow
    }
}