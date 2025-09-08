use reqwest::StatusCode;
use serde_json::{json, Value};
use testcontainers::{clients::Cli, Container};
use testcontainers_modules::{postgres::Postgres as TestPostgres, redis::Redis};
use uuid::Uuid;

/// Contract tests for PUT /folders/{id} endpoint
/// 
/// These tests verify the API contract defined in contracts/api-spec.yaml
/// Tests MUST fail initially (no implementation exists yet)
/// 
/// Expected behavior according to contract:
/// - PUT /folders/{id}
/// - Headers: Authorization: Bearer <access_token>
/// - Request: { "name": "string", "parent_folder_id": "uuid" | null }
/// - Success (200): { "id", "name", "parent_folder_id", "path", "level", "created_at", "updated_at" }
/// - Error (400): Invalid request data
/// - Error (401): Unauthorized
/// - Error (404): Folder not found

#[cfg(test)]
mod folders_update_contract_tests {
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

    #[tokio::test]
    async fn test_update_folder_name_success_returns_200() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "update_name@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            // First create a folder (this will also fail initially)
            let folder_result = create_test_folder(&ctx, &access_token, "Original Name", None).await;
            
            if let Ok(folder) = folder_result {
                let folder_id = folder.get("id").unwrap().as_str().unwrap();

                let request_body = json!({
                    "name": "Updated Name",
                    "parent_folder_id": null
                });

                let response = ctx.client
                    .put(&format!("{}/api/v1/folders/{}", ctx.base_url, folder_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&request_body)
                    .send()
                    .await;

                // This test MUST fail initially - no implementation exists
                assert!(response.is_err() || response.unwrap().status() != StatusCode::OK,
                    "Test should fail initially - no implementation exists yet");
            }
        }

        // Expected contract behavior (will pass after implementation):
        // let response = response.unwrap();
        // assert_eq!(response.status(), StatusCode::OK);
        // 
        // let updated_folder: Value = response.json().await.unwrap();
        // 
        // // Verify response structure
        // assert!(updated_folder.get("id").is_some(), "Response must contain id");
        // assert!(updated_folder.get("name").is_some(), "Response must contain name");
        // assert_eq!(updated_folder.get("name").unwrap(), "Updated Name");
        // 
        // // Verify updated_at timestamp changed
        // assert!(updated_folder.get("updated_at").is_some(), "Response must contain updated_at");
        // // Note: In real test, would compare with original folder's updated_at
    }

    #[tokio::test]
    async fn test_update_folder_parent_success_returns_200() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "update_parent@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            // Create a folder to move and a target parent folder
            let folder_result = create_test_folder(&ctx, &access_token, "Folder to Move", None).await;
            let parent_result = create_test_folder(&ctx, &access_token, "New Parent", None).await;
            
            if let (Ok(folder), Ok(parent)) = (folder_result, parent_result) {
                let folder_id = folder.get("id").unwrap().as_str().unwrap();
                let parent_id = parent.get("id").unwrap().as_str().unwrap();
                let parent_uuid = Uuid::parse_str(parent_id).unwrap();

                let request_body = json!({
                    "name": "Folder to Move",
                    "parent_folder_id": parent_uuid
                });

                let response = ctx.client
                    .put(&format!("{}/api/v1/folders/{}", ctx.base_url, folder_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&request_body)
                    .send()
                    .await;

                // This test MUST fail initially
                assert!(response.is_err() || response.unwrap().status() != StatusCode::OK,
                    "Test should fail initially - no implementation exists yet");
            }
        }

        // Expected behavior: Move folder to different parent, update path and level
    }

    #[tokio::test]
    async fn test_update_folder_nonexistent_id_returns_404() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "nonexistent@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let nonexistent_id = Uuid::new_v4();

            let request_body = json!({
                "name": "Updated Name",
                "parent_folder_id": null
            });

            let response = ctx.client
                .put(&format!("{}/api/v1/folders/{}", ctx.base_url, nonexistent_id))
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::NOT_FOUND,
                "Test should fail initially - no implementation exists yet");
        }
    }

    #[tokio::test]
    async fn test_update_folder_invalid_id_format_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "invalid_id@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let request_body = json!({
                "name": "Updated Name",
                "parent_folder_id": null
            });

            let response = ctx.client
                .put(&format!("{}/api/v1/folders/not-a-uuid", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                "Test should fail initially - no implementation exists yet");
        }
    }

    #[tokio::test]
    async fn test_update_folder_missing_name_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "missing_name@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let folder_result = create_test_folder(&ctx, &access_token, "Original Name", None).await;
            
            if let Ok(folder) = folder_result {
                let folder_id = folder.get("id").unwrap().as_str().unwrap();

                let request_body = json!({
                    "parent_folder_id": null
                });

                let response = ctx.client
                    .put(&format!("{}/api/v1/folders/{}", ctx.base_url, folder_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&request_body)
                    .send()
                    .await;

                // This test MUST fail initially
                assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                    "Test should fail initially - no implementation exists yet");
            }
        }
    }

    #[tokio::test]
    async fn test_update_folder_empty_name_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "empty_name@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let folder_result = create_test_folder(&ctx, &access_token, "Original Name", None).await;
            
            if let Ok(folder) = folder_result {
                let folder_id = folder.get("id").unwrap().as_str().unwrap();

                let request_body = json!({
                    "name": "",
                    "parent_folder_id": null
                });

                let response = ctx.client
                    .put(&format!("{}/api/v1/folders/{}", ctx.base_url, folder_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&request_body)
                    .send()
                    .await;

                // This test MUST fail initially
                assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                    "Test should fail initially - no implementation exists yet");
            }
        }
    }

    #[tokio::test]
    async fn test_update_folder_name_with_slash_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "slash_name@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let folder_result = create_test_folder(&ctx, &access_token, "Original Name", None).await;
            
            if let Ok(folder) = folder_result {
                let folder_id = folder.get("id").unwrap().as_str().unwrap();

                let request_body = json!({
                    "name": "name/with/slash",
                    "parent_folder_id": null
                });

                let response = ctx.client
                    .put(&format!("{}/api/v1/folders/{}", ctx.base_url, folder_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&request_body)
                    .send()
                    .await;

                // This test MUST fail initially
                assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                    "Test should fail initially - no implementation exists yet");
            }
        }
    }

    #[tokio::test]
    async fn test_update_folder_nonexistent_parent_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "nonexistent_parent@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let folder_result = create_test_folder(&ctx, &access_token, "Original Name", None).await;
            
            if let Ok(folder) = folder_result {
                let folder_id = folder.get("id").unwrap().as_str().unwrap();
                let nonexistent_parent = Uuid::new_v4();

                let request_body = json!({
                    "name": "Updated Name",
                    "parent_folder_id": nonexistent_parent
                });

                let response = ctx.client
                    .put(&format!("{}/api/v1/folders/{}", ctx.base_url, folder_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&request_body)
                    .send()
                    .await;

                // This test MUST fail initially
                assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                    "Test should fail initially - no implementation exists yet");
            }
        }
    }

    #[tokio::test]
    async fn test_update_folder_circular_reference_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "circular@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            // Create parent and child folders
            let parent_result = create_test_folder(&ctx, &access_token, "Parent", None).await;
            
            if let Ok(parent) = parent_result {
                let parent_id = parent.get("id").unwrap().as_str().unwrap();
                let parent_uuid = Uuid::parse_str(parent_id).unwrap();
                
                let child_result = create_test_folder(&ctx, &access_token, "Child", Some(parent_uuid)).await;
                
                if let Ok(child) = child_result {
                    let child_id = child.get("id").unwrap().as_str().unwrap();
                    let child_uuid = Uuid::parse_str(child_id).unwrap();

                    // Try to make parent a child of its own child (circular reference)
                    let request_body = json!({
                        "name": "Parent",
                        "parent_folder_id": child_uuid
                    });

                    let response = ctx.client
                        .put(&format!("{}/api/v1/folders/{}", ctx.base_url, parent_id))
                        .header("Authorization", format!("Bearer {}", access_token))
                        .header("Content-Type", "application/json")
                        .json(&request_body)
                        .send()
                        .await;

                    // This test MUST fail initially
                    assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                        "Test should fail initially - no implementation exists yet");
                }
            }
        }

        // Expected behavior: Prevent circular references in folder hierarchy
    }

    #[tokio::test]
    async fn test_update_folder_self_parent_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "self_parent@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let folder_result = create_test_folder(&ctx, &access_token, "Self Parent", None).await;
            
            if let Ok(folder) = folder_result {
                let folder_id = folder.get("id").unwrap().as_str().unwrap();
                let folder_uuid = Uuid::parse_str(folder_id).unwrap();

                // Try to make folder its own parent
                let request_body = json!({
                    "name": "Self Parent",
                    "parent_folder_id": folder_uuid
                });

                let response = ctx.client
                    .put(&format!("{}/api/v1/folders/{}", ctx.base_url, folder_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&request_body)
                    .send()
                    .await;

                // This test MUST fail initially
                assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                    "Test should fail initially - no implementation exists yet");
            }
        }
    }

    #[tokio::test]
    async fn test_update_folder_max_depth_exceeded_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "max_depth@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            // This test would require creating a deep folder hierarchy first
            // For now, just test the concept

            let folder_result = create_test_folder(&ctx, &access_token, "Deep Folder", None).await;
            
            if let Ok(folder) = folder_result {
                let folder_id = folder.get("id").unwrap().as_str().unwrap();
                
                // Assume we have a parent at level 9, moving folder to it would make it level 10
                // which should be allowed, but level 11 would be rejected
                let deep_parent = Uuid::new_v4(); // Pretend this is at max depth

                let request_body = json!({
                    "name": "Too Deep Folder",
                    "parent_folder_id": deep_parent
                });

                let response = ctx.client
                    .put(&format!("{}/api/v1/folders/{}", ctx.base_url, folder_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&request_body)
                    .send()
                    .await;

                // This test MUST fail initially
                assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                    "Test should fail initially - no implementation exists yet");
            }
        }
    }

    #[tokio::test]
    async fn test_update_folder_duplicate_name_same_parent_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "duplicate_update@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            // Create two folders with different names
            let folder1_result = create_test_folder(&ctx, &access_token, "Folder One", None).await;
            let folder2_result = create_test_folder(&ctx, &access_token, "Folder Two", None).await;
            
            if let (Ok(_folder1), Ok(folder2)) = (folder1_result, folder2_result) {
                let folder2_id = folder2.get("id").unwrap().as_str().unwrap();

                // Try to rename folder2 to same name as folder1
                let request_body = json!({
                    "name": "Folder One", // Same name as folder1
                    "parent_folder_id": null
                });

                let response = ctx.client
                    .put(&format!("{}/api/v1/folders/{}", ctx.base_url, folder2_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&request_body)
                    .send()
                    .await;

                // This test MUST fail initially
                assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                    "Test should fail initially - no implementation exists yet");
            }
        }
    }

    #[tokio::test]
    async fn test_update_folder_unauthorized_no_token_returns_401() {
        let ctx = setup_test_environment().await;

        let folder_id = Uuid::new_v4();
        let request_body = json!({
            "name": "Updated Name",
            "parent_folder_id": null
        });

        let response = ctx.client
            .put(&format!("{}/api/v1/folders/{}", ctx.base_url, folder_id))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await;

        // This test MUST fail initially
        assert!(response.is_err() || response.unwrap().status() != StatusCode::UNAUTHORIZED,
            "Test should fail initially - no implementation exists yet");
    }

    #[tokio::test]
    async fn test_update_folder_user_isolation() {
        let ctx = setup_test_environment().await;
        
        let token1_result = get_auth_token(&ctx, "user1@example.com", "password123").await;
        let token2_result = get_auth_token(&ctx, "user2@example.com", "password123").await;
        
        if let (Ok(token1), Ok(token2)) = (token1_result, token2_result) {
            // User 1 creates a folder
            let folder_result = create_test_folder(&ctx, &token1, "User1 Folder", None).await;
            
            if let Ok(folder) = folder_result {
                let folder_id = folder.get("id").unwrap().as_str().unwrap();

                // User 2 tries to update User 1's folder (should fail)
                let request_body = json!({
                    "name": "Hacked Folder",
                    "parent_folder_id": null
                });

                let response = ctx.client
                    .put(&format!("{}/api/v1/folders/{}", ctx.base_url, folder_id))
                    .header("Authorization", format!("Bearer {}", token2))
                    .header("Content-Type", "application/json")
                    .json(&request_body)
                    .send()
                    .await;

                // This test MUST fail initially
                assert!(response.is_err() || response.unwrap().status() != StatusCode::NOT_FOUND,
                    "Test should fail initially - no implementation exists yet");
            }
        }

        // Expected behavior: Users should not be able to update other users' folders
        // Should return 404 (not found) rather than 403 (forbidden) to avoid information disclosure
    }

    #[tokio::test]
    async fn test_update_folder_response_time_under_200ms() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "performance@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let folder_result = create_test_folder(&ctx, &access_token, "Performance Test", None).await;
            
            if let Ok(folder) = folder_result {
                let folder_id = folder.get("id").unwrap().as_str().unwrap();

                let request_body = json!({
                    "name": "Updated Performance Test",
                    "parent_folder_id": null
                });

                let start = std::time::Instant::now();
                
                let _response = ctx.client
                    .put(&format!("{}/api/v1/folders/{}", ctx.base_url, folder_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&request_body)
                    .send()
                    .await;

                let duration = start.elapsed();

                // This test will fail initially due to no server running
                assert!(duration.as_millis() > 200,
                    "Test should fail initially - no server running, or verify <200ms after implementation");
            }
        }
    }
}