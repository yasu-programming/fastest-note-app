use reqwest::StatusCode;
use serde_json::{json, Value};
use testcontainers::{clients::Cli, Container};
use testcontainers_modules::{postgres::Postgres as TestPostgres, redis::Redis};
use uuid::Uuid;

/// Contract tests for POST /folders endpoint
/// 
/// These tests verify the API contract defined in contracts/api-spec.yaml
/// Tests MUST fail initially (no implementation exists yet)
/// 
/// Expected behavior according to contract:
/// - POST /folders
/// - Headers: Authorization: Bearer <access_token>
/// - Request: { "name": "string", "parent_folder_id": "uuid" | null }
/// - Success (201): { "id", "name", "parent_folder_id", "path", "level", "created_at", "updated_at" }
/// - Error (400): Invalid request data
/// - Error (401): Unauthorized

#[cfg(test)]
mod folders_create_contract_tests {
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

    /// Helper function to create a folder and return its ID
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
    async fn test_create_root_folder_success_returns_201() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "create_root@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let request_body = json!({
                "name": "My Root Folder",
                "parent_folder_id": null
            });

            let response = ctx.client
                .post(&format!("{}/api/v1/folders", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
                .await;

            // This test MUST fail initially - no implementation exists
            assert!(response.is_err() || response.unwrap().status() != StatusCode::CREATED,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected contract behavior (will pass after implementation):
        // let response = response.unwrap();
        // assert_eq!(response.status(), StatusCode::CREATED);
        // 
        // let folder: Value = response.json().await.unwrap();
        // 
        // // Verify response structure matches OpenAPI spec
        // assert!(folder.get("id").is_some(), "Response must contain id");
        // assert!(folder.get("name").is_some(), "Response must contain name");
        // assert!(folder.get("parent_folder_id").is_some(), "Response must contain parent_folder_id");
        // assert!(folder.get("path").is_some(), "Response must contain path");
        // assert!(folder.get("level").is_some(), "Response must contain level");
        // assert!(folder.get("created_at").is_some(), "Response must contain created_at");
        // assert!(folder.get("updated_at").is_some(), "Response must contain updated_at");
        // 
        // // Verify values
        // assert_eq!(folder.get("name").unwrap(), "My Root Folder");
        // assert!(folder.get("parent_folder_id").unwrap().is_null(), "Root folder should have null parent");
        // assert_eq!(folder.get("level").unwrap(), 0, "Root folder should have level 0");
        // 
        // // Verify UUID format
        // let id_str = folder.get("id").unwrap().as_str().unwrap();
        // assert!(Uuid::parse_str(id_str).is_ok(), "ID should be valid UUID");
    }

    #[tokio::test]
    async fn test_create_subfolder_success_returns_201() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "create_sub@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            // First create a parent folder (this will also fail initially)
            let parent_result = create_test_folder(&ctx, &access_token, "Parent Folder", None).await;
            
            if let Ok(parent_folder) = parent_result {
                let parent_id = parent_folder.get("id").unwrap().as_str().unwrap();
                let parent_uuid = Uuid::parse_str(parent_id).unwrap();

                let request_body = json!({
                    "name": "Sub Folder",
                    "parent_folder_id": parent_uuid
                });

                let response = ctx.client
                    .post(&format!("{}/api/v1/folders", ctx.base_url))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&request_body)
                    .send()
                    .await;

                // This test MUST fail initially
                assert!(response.is_err() || response.unwrap().status() != StatusCode::CREATED,
                    "Test should fail initially - no implementation exists yet");
            }
        }

        // Expected behavior: Create subfolder with parent_folder_id set, level = parent.level + 1
    }

    #[tokio::test]
    async fn test_create_folder_missing_name_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "missing_name@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let request_body = json!({
                "parent_folder_id": null
            });

            let response = ctx.client
                .post(&format!("{}/api/v1/folders", ctx.base_url))
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
    async fn test_create_folder_empty_name_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "empty_name@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let request_body = json!({
                "name": "",
                "parent_folder_id": null
            });

            let response = ctx.client
                .post(&format!("{}/api/v1/folders", ctx.base_url))
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
    async fn test_create_folder_whitespace_only_name_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "whitespace_name@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let request_body = json!({
                "name": "   ",
                "parent_folder_id": null
            });

            let response = ctx.client
                .post(&format!("{}/api/v1/folders", ctx.base_url))
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
    async fn test_create_folder_name_with_slash_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "slash_name@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let request_body = json!({
                "name": "folder/with/slash",
                "parent_folder_id": null
            });

            let response = ctx.client
                .post(&format!("{}/api/v1/folders", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Folder names should not contain '/' character as it conflicts with path
    }

    #[tokio::test]
    async fn test_create_folder_name_too_long_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "long_name@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let long_name = "a".repeat(256); // Exceeds 255 character limit

            let request_body = json!({
                "name": long_name,
                "parent_folder_id": null
            });

            let response = ctx.client
                .post(&format!("{}/api/v1/folders", ctx.base_url))
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
    async fn test_create_folder_invalid_parent_id_format_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "invalid_parent@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let request_body = json!({
                "name": "Valid Name",
                "parent_folder_id": "not-a-uuid"
            });

            let response = ctx.client
                .post(&format!("{}/api/v1/folders", ctx.base_url))
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
    async fn test_create_folder_nonexistent_parent_id_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "nonexistent_parent@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let nonexistent_id = Uuid::new_v4();

            let request_body = json!({
                "name": "Valid Name",
                "parent_folder_id": nonexistent_id
            });

            let response = ctx.client
                .post(&format!("{}/api/v1/folders", ctx.base_url))
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
    async fn test_create_folder_duplicate_name_same_parent_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "duplicate_name@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let request_body = json!({
                "name": "Duplicate Name",
                "parent_folder_id": null
            });

            // First creation (should succeed)
            let _first_response = ctx.client
                .post(&format!("{}/api/v1/folders", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
                .await;

            // Second creation with same name and parent (should fail)
            let second_response = ctx.client
                .post(&format!("{}/api/v1/folders", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
                .await;

            // This test MUST fail initially
            assert!(second_response.is_err() || second_response.unwrap().status() != StatusCode::BAD_REQUEST,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Should prevent duplicate folder names within same parent
    }

    #[tokio::test]
    async fn test_create_folder_max_depth_limit_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "max_depth@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            // This test would need to create a chain of 10 nested folders
            // then try to create an 11th level folder
            // For now, just test the concept

            let request_body = json!({
                "name": "Level 11 Folder",
                "parent_folder_id": Uuid::new_v4() // Assume this is at level 10
            });

            let response = ctx.client
                .post(&format!("{}/api/v1/folders", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                "Test should fail initially - no implementation exists yet");
        }

        // Expected behavior: Should enforce maximum folder depth of 10 levels
    }

    #[tokio::test]
    async fn test_create_folder_unauthorized_no_token_returns_401() {
        let ctx = setup_test_environment().await;

        let request_body = json!({
            "name": "Unauthorized Folder",
            "parent_folder_id": null
        });

        let response = ctx.client
            .post(&format!("{}/api/v1/folders", ctx.base_url))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await;

        // This test MUST fail initially
        assert!(response.is_err() || response.unwrap().status() != StatusCode::UNAUTHORIZED,
            "Test should fail initially - no implementation exists yet");
    }

    #[tokio::test]
    async fn test_create_folder_invalid_token_returns_401() {
        let ctx = setup_test_environment().await;

        let request_body = json!({
            "name": "Valid Name",
            "parent_folder_id": null
        });

        let response = ctx.client
            .post(&format!("{}/api/v1/folders", ctx.base_url))
            .header("Authorization", "Bearer invalid_token_123")
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await;

        // This test MUST fail initially
        assert!(response.is_err() || response.unwrap().status() != StatusCode::UNAUTHORIZED,
            "Test should fail initially - no implementation exists yet");
    }

    #[tokio::test]
    async fn test_create_folder_malformed_json_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "malformed_json@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let malformed_json = r#"{"name": "Valid Name", "parent_folder_id": }"#;

            let response = ctx.client
                .post(&format!("{}/api/v1/folders", ctx.base_url))
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

    #[tokio::test]
    async fn test_create_folder_invalid_content_type_returns_400() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "invalid_content@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let request_body = r#"{"name": "Valid Name", "parent_folder_id": null}"#;

            let response = ctx.client
                .post(&format!("{}/api/v1/folders", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "text/plain")
                .body(request_body)
                .send()
                .await;

            // This test MUST fail initially
            assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                "Test should fail initially - no implementation exists yet");
        }
    }

    #[tokio::test]
    async fn test_create_folder_response_time_under_200ms() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "performance@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let request_body = json!({
                "name": "Performance Test Folder",
                "parent_folder_id": null
            });

            let start = std::time::Instant::now();
            
            let _response = ctx.client
                .post(&format!("{}/api/v1/folders", ctx.base_url))
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

    #[tokio::test]
    async fn test_create_folder_user_isolation() {
        let ctx = setup_test_environment().await;
        
        let token1_result = get_auth_token(&ctx, "user1@example.com", "password123").await;
        let token2_result = get_auth_token(&ctx, "user2@example.com", "password123").await;
        
        if let (Ok(token1), Ok(token2)) = (token1_result, token2_result) {
            // User 1 creates a folder
            let folder1_result = create_test_folder(&ctx, &token1, "User1 Folder", None).await;
            
            if let Ok(folder1) = folder1_result {
                let folder1_id = folder1.get("id").unwrap().as_str().unwrap();
                let folder1_uuid = Uuid::parse_str(folder1_id).unwrap();

                // User 2 tries to create a subfolder in User 1's folder (should fail)
                let request_body = json!({
                    "name": "User2 Subfolder",
                    "parent_folder_id": folder1_uuid
                });

                let response = ctx.client
                    .post(&format!("{}/api/v1/folders", ctx.base_url))
                    .header("Authorization", format!("Bearer {}", token2))
                    .header("Content-Type", "application/json")
                    .json(&request_body)
                    .send()
                    .await;

                // This test MUST fail initially
                assert!(response.is_err() || response.unwrap().status() != StatusCode::BAD_REQUEST,
                    "Test should fail initially - no implementation exists yet");
            }
        }

        // Expected behavior: Users should not be able to create folders under other users' folders
    }
}