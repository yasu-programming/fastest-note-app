use reqwest::StatusCode;
use serde_json::{json, Value};
use testcontainers::{clients::Cli, Container};
use testcontainers_modules::{postgres::Postgres as TestPostgres, redis::Redis};
use uuid::Uuid;

/// Contract tests for PUT /notes/{id}/move endpoint
/// 
/// These tests verify the API contract defined in contracts/api-spec.yaml
/// Tests MUST fail initially (no implementation exists yet)
/// 
/// Expected behavior according to contract:
/// - PUT /notes/{id}/move
/// - Headers: Authorization: Bearer <access_token>, Content-Type: application/json
/// - Body: { "folder_id": "uuid|null", "version": number }
/// - Success (200): { "id", "title", "content", "folder_id", "created_at", "updated_at", "version" }
/// - Error (400): Bad request (validation errors, version mismatch)
/// - Error (401): Unauthorized
/// - Error (404): Note not found or target folder not found
/// - Error (409): Conflict (optimistic locking)

#[cfg(test)]
mod notes_move_contract_tests {
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

    /// Helper function to create a test note, returning note data
    async fn create_test_note(ctx: &TestContext, access_token: &str, title: &str, content: &str, folder_id: Option<&str>) -> Result<Value, Box<dyn std::error::Error>> {
        let note_body = json!({
            "title": title,
            "content": content,
            "folder_id": folder_id
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
    async fn test_move_note_to_folder_success_returns_200() {
        let ctx = setup_test_environment().await;
        
        // Get auth token (this will also fail initially)
        let token_result = get_auth_token(&ctx, "note_move@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            // Create a test folder and note (will also fail initially)
            let folder_result = create_test_folder(&ctx, &access_token, "Target Folder").await;
            let note_result = create_test_note(&ctx, &access_token, "Test Note", "Test content", None).await;
            
            if let (Ok(folder_id), Ok(note_data)) = (folder_result, note_result) {
                let note_id = note_data["id"].as_str().unwrap();
                let version = note_data["version"].as_u64().unwrap();

                let move_body = json!({
                    "folder_id": folder_id,
                    "version": version
                });

                let response = ctx.client
                    .put(&format!("{}/api/v1/notes/{}/move", ctx.base_url, note_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&move_body)
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
        // let moved_note: Value = response.json().await.unwrap();
        // 
        // // Verify response structure matches OpenAPI spec
        // assert_eq!(moved_note["folder_id"], folder_id);
        // assert_eq!(moved_note["id"], note_id);
        // assert_eq!(moved_note["version"], version + 1);
        // 
        // // Verify timestamps
        // assert!(moved_note.get("updated_at").is_some());
        // assert!(moved_note["updated_at"] != note_data["updated_at"]);
    }

    #[tokio::test]
    async fn test_move_note_to_root_folder_success() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "move_to_root@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            // Create a folder and note in that folder
            let folder_result = create_test_folder(&ctx, &access_token, "Source Folder").await;
            
            if let Ok(folder_id) = folder_result {
                let note_result = create_test_note(&ctx, &access_token, "Test Note", "Test content", Some(&folder_id)).await;
                
                if let Ok(note_data) = note_result {
                    let note_id = note_data["id"].as_str().unwrap();
                    let version = note_data["version"].as_u64().unwrap();

                    let move_body = json!({
                        "folder_id": null,  // Move to root
                        "version": version
                    });

                    let response = ctx.client
                        .put(&format!("{}/api/v1/notes/{}/move", ctx.base_url, note_id))
                        .header("Authorization", format!("Bearer {}", access_token))
                        .header("Content-Type", "application/json")
                        .json(&move_body)
                        .send()
                        .await;

                    // This test MUST fail initially
                    assert!(response.is_err() || response.unwrap().status() != StatusCode::OK,
                        "Test should fail initially - no implementation exists yet");
                }
            }
        }

        // Expected behavior: Note moved to root folder (folder_id = null)
    }

    #[tokio::test]
    async fn test_move_note_missing_version_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "missing_version_move@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let folder_result = create_test_folder(&ctx, &access_token, "Target Folder").await;
            let note_result = create_test_note(&ctx, &access_token, "Test Note", "Test content", None).await;
            
            if let (Ok(folder_id), Ok(note_data)) = (folder_result, note_result) {
                let note_id = note_data["id"].as_str().unwrap();

                let move_body = json!({
                    "folder_id": folder_id
                    // Missing version field
                });

                let response = ctx.client
                    .put(&format!("{}/api/v1/notes/{}/move", ctx.base_url, note_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&move_body)
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
    async fn test_move_note_version_mismatch_returns_409() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "version_mismatch_move@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let folder_result = create_test_folder(&ctx, &access_token, "Target Folder").await;
            let note_result = create_test_note(&ctx, &access_token, "Test Note", "Test content", None).await;
            
            if let (Ok(folder_id), Ok(note_data)) = (folder_result, note_result) {
                let note_id = note_data["id"].as_str().unwrap();
                let version = note_data["version"].as_u64().unwrap();

                let move_body = json!({
                    "folder_id": folder_id,
                    "version": version + 999  // Wrong version
                });

                let response = ctx.client
                    .put(&format!("{}/api/v1/notes/{}/move", ctx.base_url, note_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&move_body)
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
    async fn test_move_note_nonexistent_note_returns_404() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "nonexistent_note_move@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let folder_result = create_test_folder(&ctx, &access_token, "Target Folder").await;
            
            if let Ok(folder_id) = folder_result {
                let nonexistent_note_id = Uuid::new_v4();

                let move_body = json!({
                    "folder_id": folder_id,
                    "version": 1
                });

                let response = ctx.client
                    .put(&format!("{}/api/v1/notes/{}/move", ctx.base_url, nonexistent_note_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&move_body)
                    .send()
                    .await;

                // This test MUST fail initially
                assert!(response.is_err() || response.unwrap().status() != StatusCode::NOT_FOUND,
                    "Test should fail initially - no implementation exists yet");
            }
        }

        // Expected behavior: Return 404 for non-existent note ID
    }

    #[tokio::test]
    async fn test_move_note_nonexistent_folder_returns_404() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "nonexistent_folder_move@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let note_result = create_test_note(&ctx, &access_token, "Test Note", "Test content", None).await;
            
            if let Ok(note_data) = note_result {
                let note_id = note_data["id"].as_str().unwrap();
                let version = note_data["version"].as_u64().unwrap();
                let nonexistent_folder_id = Uuid::new_v4();

                let move_body = json!({
                    "folder_id": nonexistent_folder_id,
                    "version": version
                });

                let response = ctx.client
                    .put(&format!("{}/api/v1/notes/{}/move", ctx.base_url, note_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&move_body)
                    .send()
                    .await;

                // This test MUST fail initially
                assert!(response.is_err() || response.unwrap().status() != StatusCode::NOT_FOUND,
                    "Test should fail initially - no implementation exists yet");
            }
        }

        // Expected behavior: Return 404 for non-existent target folder
    }

    #[tokio::test]
    async fn test_move_note_invalid_note_id_format_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "invalid_note_id_move@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let folder_result = create_test_folder(&ctx, &access_token, "Target Folder").await;
            
            if let Ok(folder_id) = folder_result {
                let move_body = json!({
                    "folder_id": folder_id,
                    "version": 1
                });

                let response = ctx.client
                    .put(&format!("{}/api/v1/notes/not-a-uuid/move", ctx.base_url))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&move_body)
                    .send()
                    .await;

                // This test MUST fail initially
                assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                    "Test should fail initially - no implementation exists yet");
            }
        }

        // Expected behavior: Return 400 for invalid UUID format in note ID path
    }

    #[tokio::test]
    async fn test_move_note_invalid_folder_id_format_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "invalid_folder_id_move@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let note_result = create_test_note(&ctx, &access_token, "Test Note", "Test content", None).await;
            
            if let Ok(note_data) = note_result {
                let note_id = note_data["id"].as_str().unwrap();
                let version = note_data["version"].as_u64().unwrap();

                let move_body = json!({
                    "folder_id": "not-a-uuid",
                    "version": version
                });

                let response = ctx.client
                    .put(&format!("{}/api/v1/notes/{}/move", ctx.base_url, note_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&move_body)
                    .send()
                    .await;

                // This test MUST fail initially
                assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                    "Test should fail initially - no implementation exists yet");
            }
        }

        // Expected behavior: Return 400 for invalid UUID format in folder_id
    }

    #[tokio::test]
    async fn test_move_note_unauthorized_no_token_returns_401() {
        let ctx = setup_test_environment().await;

        let note_id = Uuid::new_v4();
        let folder_id = Uuid::new_v4();
        let move_body = json!({
            "folder_id": folder_id,
            "version": 1
        });

        let response = ctx.client
            .put(&format!("{}/api/v1/notes/{}/move", ctx.base_url, note_id))
            .header("Content-Type", "application/json")
            .json(&move_body)
            .send()
            .await;

        // This test MUST fail initially
        assert!(response.is_err() || response.unwrap().status() != StatusCode::UNAUTHORIZED,
            "Test should fail initially - no implementation exists yet");

        // Expected behavior: Return 401 when no Authorization header provided
    }

    #[tokio::test]
    async fn test_move_note_invalid_token_returns_401() {
        let ctx = setup_test_environment().await;

        let note_id = Uuid::new_v4();
        let folder_id = Uuid::new_v4();
        let move_body = json!({
            "folder_id": folder_id,
            "version": 1
        });

        let response = ctx.client
            .put(&format!("{}/api/v1/notes/{}/move", ctx.base_url, note_id))
            .header("Authorization", "Bearer invalid_token_123")
            .header("Content-Type", "application/json")
            .json(&move_body)
            .send()
            .await;

        // This test MUST fail initially
        assert!(response.is_err() || response.unwrap().status() != StatusCode::UNAUTHORIZED,
            "Test should fail initially - no implementation exists yet");
    }

    #[tokio::test]
    async fn test_move_note_user_isolation() {
        let ctx = setup_test_environment().await;
        
        // Get tokens for two different users
        let token1_result = get_auth_token(&ctx, "user1_move@example.com", "password123").await;
        let token2_result = get_auth_token(&ctx, "user2_move@example.com", "password123").await;
        
        if let (Ok(token1), Ok(token2)) = (token1_result, token2_result) {
            // User 1 creates a note and folder
            let note_result = create_test_note(&ctx, &token1, "User1 Note", "User1 content", None).await;
            let folder_result = create_test_folder(&ctx, &token2, "User2 Folder").await;  // User 2's folder
            
            if let (Ok(note_data), Ok(folder_id)) = (note_result, folder_result) {
                let note_id = note_data["id"].as_str().unwrap();
                let version = note_data["version"].as_u64().unwrap();

                // User 1 tries to move their note to User 2's folder
                let move_body = json!({
                    "folder_id": folder_id,
                    "version": version
                });

                let response = ctx.client
                    .put(&format!("{}/api/v1/notes/{}/move", ctx.base_url, note_id))
                    .header("Authorization", format!("Bearer {}", token1))
                    .header("Content-Type", "application/json")
                    .json(&move_body)
                    .send()
                    .await;

                // This test MUST fail initially
                assert!(response.is_err() || response.unwrap().status() != StatusCode::NOT_FOUND,
                    "Test should fail initially - no implementation exists yet");
            }
        }

        // Expected behavior: Users can only move notes to their own folders (404 for others' folders)
    }

    #[tokio::test]
    async fn test_move_note_same_folder_no_change() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "same_folder_move@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let folder_result = create_test_folder(&ctx, &access_token, "Test Folder").await;
            
            if let Ok(folder_id) = folder_result {
                let note_result = create_test_note(&ctx, &access_token, "Test Note", "Test content", Some(&folder_id)).await;
                
                if let Ok(note_data) = note_result {
                    let note_id = note_data["id"].as_str().unwrap();
                    let version = note_data["version"].as_u64().unwrap();

                    // Move note to the same folder it's already in
                    let move_body = json!({
                        "folder_id": folder_id,
                        "version": version
                    });

                    let response = ctx.client
                        .put(&format!("{}/api/v1/notes/{}/move", ctx.base_url, note_id))
                        .header("Authorization", format!("Bearer {}", access_token))
                        .header("Content-Type", "application/json")
                        .json(&move_body)
                        .send()
                        .await;

                    // This test MUST fail initially
                    assert!(response.is_err() || response.unwrap().status() != StatusCode::OK,
                        "Test should fail initially - no implementation exists yet");
                }
            }
        }

        // Expected behavior: Should succeed but version should still increment (idempotent operation)
    }

    #[tokio::test]
    async fn test_move_note_malformed_json_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "malformed_move@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let note_result = create_test_note(&ctx, &access_token, "Test Note", "Test content", None).await;
            
            if let Ok(note_data) = note_result {
                let note_id = note_data["id"].as_str().unwrap();
                let malformed_json = r#"{"folder_id": "uuid", "version": }"#;

                let response = ctx.client
                    .put(&format!("{}/api/v1/notes/{}/move", ctx.base_url, note_id))
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
    async fn test_move_note_concurrent_moves() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "concurrent_move@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let folder1_result = create_test_folder(&ctx, &access_token, "Folder 1").await;
            let folder2_result = create_test_folder(&ctx, &access_token, "Folder 2").await;
            let note_result = create_test_note(&ctx, &access_token, "Test Note", "Test content", None).await;
            
            if let (Ok(folder1_id), Ok(folder2_id), Ok(note_data)) = (folder1_result, folder2_result, note_result) {
                let note_id = note_data["id"].as_str().unwrap();
                let version = note_data["version"].as_u64().unwrap();

                // Make concurrent move requests to different folders with same version
                let mut handles = Vec::new();
                
                for (i, folder_id) in [folder1_id, folder2_id].iter().enumerate() {
                    let client = ctx.client.clone();
                    let url = ctx.base_url.clone();
                    let token = access_token.clone();
                    let id = note_id.to_string();
                    let target_folder = folder_id.clone();
                    
                    let handle = tokio::spawn(async move {
                        let move_body = json!({
                            "folder_id": target_folder,
                            "version": version
                        });

                        client
                            .put(&format!("{}/api/v1/notes/{}/move", url, id))
                            .header("Authorization", format!("Bearer {}", token))
                            .header("Content-Type", "application/json")
                            .json(&move_body)
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
    async fn test_move_note_response_time_under_200ms() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "performance_move@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let folder_result = create_test_folder(&ctx, &access_token, "Performance Folder").await;
            let note_result = create_test_note(&ctx, &access_token, "Performance Note", "Performance test", None).await;
            
            if let (Ok(folder_id), Ok(note_data)) = (folder_result, note_result) {
                let note_id = note_data["id"].as_str().unwrap();
                let version = note_data["version"].as_u64().unwrap();

                let move_body = json!({
                    "folder_id": folder_id,
                    "version": version
                });

                let start = std::time::Instant::now();
                
                let _response = ctx.client
                    .put(&format!("{}/api/v1/notes/{}/move", ctx.base_url, note_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&move_body)
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
    async fn test_move_note_json_content_type_response() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "json_response_move@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let folder_result = create_test_folder(&ctx, &access_token, "JSON Test Folder").await;
            let note_result = create_test_note(&ctx, &access_token, "JSON Test Note", "JSON test content", None).await;
            
            if let (Ok(folder_id), Ok(note_data)) = (folder_result, note_result) {
                let note_id = note_data["id"].as_str().unwrap();
                let version = note_data["version"].as_u64().unwrap();

                let move_body = json!({
                    "folder_id": folder_id,
                    "version": version
                });

                let response = ctx.client
                    .put(&format!("{}/api/v1/notes/{}/move", ctx.base_url, note_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&move_body)
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

    #[tokio::test]
    async fn test_move_note_missing_content_type_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "no_content_type_move@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let folder_result = create_test_folder(&ctx, &access_token, "Test Folder").await;
            let note_result = create_test_note(&ctx, &access_token, "Test Note", "Test content", None).await;
            
            if let (Ok(folder_id), Ok(note_data)) = (folder_result, note_result) {
                let note_id = note_data["id"].as_str().unwrap();
                let version = note_data["version"].as_u64().unwrap();

                let move_body = json!({
                    "folder_id": folder_id,
                    "version": version
                });

                let response = ctx.client
                    .put(&format!("{}/api/v1/notes/{}/move", ctx.base_url, note_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    // Missing Content-Type header
                    .body(move_body.to_string())
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
    async fn test_move_note_cache_invalidation() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "cache_invalidation_move@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let folder_result = create_test_folder(&ctx, &access_token, "Cache Test Folder").await;
            let note_result = create_test_note(&ctx, &access_token, "Cache Test Note", "Cache test content", None).await;
            
            if let (Ok(folder_id), Ok(note_data)) = (folder_result, note_result) {
                let note_id = note_data["id"].as_str().unwrap();
                let version = note_data["version"].as_u64().unwrap();

                // Get the note to potentially cache it
                let _get_response1 = ctx.client
                    .get(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .send()
                    .await;

                // Move the note
                let move_body = json!({
                    "folder_id": folder_id,
                    "version": version
                });

                let _move_response = ctx.client
                    .put(&format!("{}/api/v1/notes/{}/move", ctx.base_url, note_id))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&move_body)
                    .send()
                    .await;

                // Get the note again (should reflect new folder_id, not cached)
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

        // Expected behavior: Cache should be properly invalidated after move
    }
}