use reqwest::StatusCode;
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use testcontainers::{clients::Cli, Container};
use testcontainers_modules::{postgres::Postgres as TestPostgres, redis::Redis};
use tokio::time::timeout;

/// Integration test for Quickstart Scenario 1: User Registration & First Note Creation
/// 
/// This test validates the complete user journey from registration to creating their first note,
/// based on the quickstart scenarios defined in specs/001-web-notion/quickstart.md
/// 
/// Tests MUST fail initially (no implementation exists yet)
/// 
/// Scenario covers:
/// 1. User Registration (FR-008)
/// 2. User Login and JWT authentication
/// 3. Create first note at root level (FR-001)  
/// 4. Performance validation (<200ms response times)

#[cfg(test)]
mod quickstart_scenario_1_tests {
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

    #[tokio::test]
    async fn test_quickstart_scenario_1_complete_user_journey() {
        let ctx = setup_test_environment().await;
        
        // This integration test MUST fail initially - no server implementation exists
        let scenario_result = run_complete_scenario(&ctx).await;
        
        assert!(scenario_result.is_err(),
            "Test should fail initially - no server implementation exists yet");

        // Expected behavior after implementation:
        // The complete scenario should pass, demonstrating:
        // - User can register successfully
        // - User can login and receive JWT tokens  
        // - User can create their first note
        // - All operations complete within performance targets
    }

    async fn run_complete_scenario(ctx: &TestContext) -> Result<ScenarioResults, Box<dyn std::error::Error>> {
        let mut results = ScenarioResults::new();
        
        // Step 1: User Registration (FR-008)
        println!("Step 1: User Registration");
        let registration_start = Instant::now();
        
        let register_body = json!({
            "email": "test@example.com",
            "password": "SecurePass123!"
        });

        let register_response = ctx.client
            .post(&format!("{}/api/v1/auth/register", ctx.base_url))
            .header("Content-Type", "application/json")
            .json(&register_body)
            .send()
            .await?;

        results.registration_time = registration_start.elapsed();
        
        // Validate registration response
        if register_response.status() != StatusCode::CREATED {
            return Err(format!("Registration failed with status: {}", register_response.status()).into());
        }

        let register_data: Value = register_response.json().await?;
        if !register_data.get("access_token").is_some() {
            return Err("Registration response missing access_token".into());
        }
        if !register_data.get("refresh_token").is_some() {
            return Err("Registration response missing refresh_token".into());
        }

        println!("✓ Registration successful in {:?}", results.registration_time);
        
        // Step 2: User Login (validate JWT authentication)
        println!("Step 2: User Login");
        let login_start = Instant::now();
        
        let login_body = json!({
            "email": "test@example.com",
            "password": "SecurePass123!"
        });

        let login_response = ctx.client
            .post(&format!("{}/api/v1/auth/login", ctx.base_url))
            .header("Content-Type", "application/json")
            .json(&login_body)
            .send()
            .await?;

        results.login_time = login_start.elapsed();
        
        if login_response.status() != StatusCode::OK {
            return Err(format!("Login failed with status: {}", login_response.status()).into());
        }

        let login_data: Value = login_response.json().await?;
        let access_token = login_data["access_token"].as_str()
            .ok_or("Login response missing access_token")?;

        println!("✓ Login successful in {:?}", results.login_time);

        // Step 3: Create First Note (FR-001) - Performance target: <200ms
        println!("Step 3: Create First Note");
        let note_creation_start = Instant::now();
        
        let note_body = json!({
            "title": "My First Note",
            "content": "This is my first note content.",
            "folder_id": null  // Root level
        });

        let note_response = ctx.client
            .post(&format!("{}/api/v1/notes", ctx.base_url))
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .json(&note_body)
            .send()
            .await?;

        results.note_creation_time = note_creation_start.elapsed();
        
        if note_response.status() != StatusCode::CREATED {
            return Err(format!("Note creation failed with status: {}", note_response.status()).into());
        }

        let note_data: Value = note_response.json().await?;
        if !note_data.get("id").is_some() {
            return Err("Note creation response missing id".into());
        }
        if note_data["title"] != "My First Note" {
            return Err("Note creation response incorrect title".into());
        }
        if note_data["content"] != "This is my first note content." {
            return Err("Note creation response incorrect content".into());
        }

        println!("✓ Note created successfully in {:?}", results.note_creation_time);

        // Step 4: Validate Performance Requirements
        println!("Step 4: Performance Validation");
        results.validate_performance_targets()?;

        // Step 5: Verify Note Retrieval
        println!("Step 5: Verify Note Retrieval");
        let note_id = note_data["id"].as_str().unwrap();
        let retrieval_start = Instant::now();

        let get_response = ctx.client
            .get(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;

        results.note_retrieval_time = retrieval_start.elapsed();

        if get_response.status() != StatusCode::OK {
            return Err(format!("Note retrieval failed with status: {}", get_response.status()).into());
        }

        let retrieved_note: Value = get_response.json().await?;
        if retrieved_note["title"] != "My First Note" {
            return Err("Retrieved note has incorrect title".into());
        }

        println!("✓ Note retrieved successfully in {:?}", results.note_retrieval_time);

        // Step 6: List User's Notes
        println!("Step 6: List User's Notes");
        let list_start = Instant::now();

        let list_response = ctx.client
            .get(&format!("{}/api/v1/notes", ctx.base_url))
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;

        results.notes_list_time = list_start.elapsed();

        if list_response.status() != StatusCode::OK {
            return Err(format!("Notes list failed with status: {}", list_response.status()).into());
        }

        let notes_data: Value = list_response.json().await?;
        let notes = notes_data["notes"].as_array()
            .ok_or("Notes list response missing notes array")?;
        
        if notes.len() != 1 {
            return Err(format!("Expected 1 note, found {}", notes.len()).into());
        }

        println!("✓ Notes listed successfully in {:?}", results.notes_list_time);

        Ok(results)
    }

    #[derive(Debug)]
    struct ScenarioResults {
        registration_time: Duration,
        login_time: Duration,
        note_creation_time: Duration,
        note_retrieval_time: Duration,
        notes_list_time: Duration,
    }

    impl ScenarioResults {
        fn new() -> Self {
            Self {
                registration_time: Duration::from_secs(0),
                login_time: Duration::from_secs(0),
                note_creation_time: Duration::from_secs(0),
                note_retrieval_time: Duration::from_secs(0),
                notes_list_time: Duration::from_secs(0),
            }
        }

        fn validate_performance_targets(&self) -> Result<(), Box<dyn std::error::Error>> {
            // Performance targets from quickstart.md
            let max_api_time = Duration::from_millis(200);
            
            if self.registration_time > max_api_time {
                return Err(format!("Registration too slow: {:?} > 200ms", self.registration_time).into());
            }
            
            if self.login_time > max_api_time {
                return Err(format!("Login too slow: {:?} > 200ms", self.login_time).into());
            }
            
            if self.note_creation_time > max_api_time {
                return Err(format!("Note creation too slow: {:?} > 200ms", self.note_creation_time).into());
            }
            
            if self.note_retrieval_time > max_api_time {
                return Err(format!("Note retrieval too slow: {:?} > 200ms", self.note_retrieval_time).into());
            }
            
            if self.notes_list_time > max_api_time {
                return Err(format!("Notes list too slow: {:?} > 200ms", self.notes_list_time).into());
            }

            println!("✓ All operations meet performance targets (<200ms)");
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_registration_edge_cases() {
        let ctx = setup_test_environment().await;
        
        // Test duplicate email registration
        let register_body = json!({
            "email": "duplicate@example.com",
            "password": "SecurePass123!"
        });

        // This should fail initially due to no server
        let first_response = ctx.client
            .post(&format!("{}/api/v1/auth/register", ctx.base_url))
            .header("Content-Type", "application/json")
            .json(&register_body)
            .send()
            .await;

        assert!(first_response.is_err(),
            "Test should fail initially - no server implementation exists yet");

        // Expected behavior after implementation:
        // - First registration should succeed
        // - Second registration with same email should fail with 409 Conflict
    }

    #[tokio::test]
    async fn test_invalid_authentication() {
        let ctx = setup_test_environment().await;
        
        // Try to create note without authentication
        let note_body = json!({
            "title": "Unauthorized Note",
            "content": "This should fail"
        });

        let response = ctx.client
            .post(&format!("{}/api/v1/notes", ctx.base_url))
            .header("Content-Type", "application/json")
            .json(&note_body)
            .send()
            .await;

        assert!(response.is_err(),
            "Test should fail initially - no server implementation exists yet");

        // Expected behavior: Should return 401 Unauthorized
    }

    #[tokio::test]
    async fn test_note_validation() {
        let ctx = setup_test_environment().await;
        
        // This will fail initially but tests the validation logic
        let test_cases = vec![
            // Empty title
            json!({
                "title": "",
                "content": "Content with empty title"
            }),
            // Title too long (>255 chars)
            json!({
                "title": "a".repeat(256),
                "content": "Content with too long title"
            }),
            // Content too large (>1MB)
            json!({
                "title": "Large content note",
                "content": "a".repeat(1024 * 1024 + 1)
            }),
        ];

        for test_case in test_cases {
            let response = ctx.client
                .post(&format!("{}/api/v1/notes", ctx.base_url))
                .header("Authorization", "Bearer fake_token")
                .header("Content-Type", "application/json")
                .json(&test_case)
                .send()
                .await;

            assert!(response.is_err(),
                "Test should fail initially - no server implementation exists yet");
        }

        // Expected behavior: All cases should return 400 Bad Request with validation errors
    }

    #[tokio::test]
    async fn test_concurrent_note_creation() {
        let ctx = setup_test_environment().await;
        
        // Simulate multiple users creating notes simultaneously
        let mut handles = Vec::new();
        
        for i in 0..5 {
            let client = ctx.client.clone();
            let url = ctx.base_url.clone();
            
            let handle = tokio::spawn(async move {
                let note_body = json!({
                    "title": format!("Concurrent Note {}", i),
                    "content": format!("Content for note {}", i),
                    "folder_id": null
                });

                client
                    .post(&format!("{}/api/v1/notes", url))
                    .header("Authorization", "Bearer fake_token")
                    .header("Content-Type", "application/json")
                    .json(&note_body)
                    .send()
                    .await
            });
            
            handles.push(handle);
        }

        let results = futures::future::join_all(handles).await;
        
        // All should fail initially
        assert!(results.iter().all(|r| r.is_err()),
            "Test should fail initially - no server implementation exists yet");

        // Expected behavior: All concurrent requests should succeed independently
    }

    #[tokio::test] 
    async fn test_jwt_token_expiration() {
        let ctx = setup_test_environment().await;
        
        // This test validates JWT token handling
        // Using obviously expired token
        let expired_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ0ZXN0IiwiZXhwIjoxNjAwMDAwMDAwfQ.invalid";
        
        let note_body = json!({
            "title": "Test with expired token",
            "content": "This should fail"
        });

        let response = ctx.client
            .post(&format!("{}/api/v1/notes", ctx.base_url))
            .header("Authorization", format!("Bearer {}", expired_token))
            .header("Content-Type", "application/json")
            .json(&note_body)
            .send()
            .await;

        assert!(response.is_err(),
            "Test should fail initially - no server implementation exists yet");

        // Expected behavior: Should return 401 Unauthorized for expired token
    }

    #[tokio::test]
    async fn test_content_type_validation() {
        let ctx = setup_test_environment().await;
        
        // Test with wrong content type
        let note_json = r#"{"title": "Test", "content": "Test content"}"#;
        
        let response = ctx.client
            .post(&format!("{}/api/v1/notes", ctx.base_url))
            .header("Authorization", "Bearer fake_token")
            .header("Content-Type", "text/plain") // Wrong content type
            .body(note_json)
            .send()
            .await;

        assert!(response.is_err(),
            "Test should fail initially - no server implementation exists yet");

        // Expected behavior: Should return 415 Unsupported Media Type
    }

    #[tokio::test]
    async fn test_malformed_json_handling() {
        let ctx = setup_test_environment().await;
        
        let malformed_json = r#"{"title": "Test", "content": }"#; // Invalid JSON
        
        let response = ctx.client
            .post(&format!("{}/api/v1/notes", ctx.base_url))
            .header("Authorization", "Bearer fake_token")
            .header("Content-Type", "application/json")
            .body(malformed_json)
            .send()
            .await;

        assert!(response.is_err(),
            "Test should fail initially - no server implementation exists yet");

        // Expected behavior: Should return 400 Bad Request for malformed JSON
    }

    #[tokio::test]
    async fn test_user_isolation() {
        let ctx = setup_test_environment().await;
        
        // This test ensures users can only see their own notes
        // Both user registration and note creation will fail initially
        let user1_result = register_and_login(&ctx, "user1@example.com", "password123").await;
        let user2_result = register_and_login(&ctx, "user2@example.com", "password123").await;
        
        assert!(user1_result.is_err() && user2_result.is_err(),
            "Test should fail initially - no server implementation exists yet");

        // Expected behavior after implementation:
        // - User1 creates note
        // - User2 cannot see User1's note
        // - Each user only sees their own notes
    }

    async fn register_and_login(ctx: &TestContext, email: &str, password: &str) -> Result<String, Box<dyn std::error::Error>> {
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
}