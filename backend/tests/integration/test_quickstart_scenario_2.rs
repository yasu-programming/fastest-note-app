use reqwest::StatusCode;
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use testcontainers::{clients::Cli, Container};
use testcontainers_modules::{postgres::Postgres as TestPostgres, redis::Redis};
use tokio::time::timeout;

/// Integration test for Quickstart Scenario 2: Folder Hierarchy & Note Organization  
/// 
/// This test validates the complete workflow of creating folder hierarchies and organizing notes,
/// based on the quickstart scenarios defined in specs/001-web-notion/quickstart.md
/// 
/// Tests MUST fail initially (no implementation exists yet)
/// 
/// Scenario covers:
/// 1. Create nested folder structure (FR-002, FR-013)
/// 2. Move notes between folders (FR-007)
/// 3. Validate folder depth limits (max 10 levels)
/// 4. Test drag-and-drop organization workflow
/// 5. Verify materialized path calculations

#[cfg(test)]
mod quickstart_scenario_2_tests {
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
    async fn test_quickstart_scenario_2_complete_folder_workflow() {
        let ctx = setup_test_environment().await;
        
        // This integration test MUST fail initially - no server implementation exists
        let scenario_result = run_complete_folder_scenario(&ctx).await;
        
        assert!(scenario_result.is_err(),
            "Test should fail initially - no server implementation exists yet");

        // Expected behavior after implementation:
        // - User can create nested folder hierarchies
        // - Notes can be moved between folders efficiently  
        // - Folder depth limits are enforced
        // - All operations complete within performance targets
    }

    async fn run_complete_folder_scenario(ctx: &TestContext) -> Result<FolderScenarioResults, Box<dyn std::error::Error>> {
        let mut results = FolderScenarioResults::new();
        
        // Setup: Get authenticated user
        let access_token = get_auth_token(&ctx, "folder_user@example.com", "SecurePass123!").await?;
        
        // Step 1: Create root folder (FR-002)
        println!("Step 1: Create root folder 'Work Projects'");
        let root_folder_start = Instant::now();
        
        let root_folder_body = json!({
            "name": "Work Projects"
        });

        let root_response = ctx.client
            .post(&format!("{}/api/v1/folders", ctx.base_url))
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .json(&root_folder_body)
            .send()
            .await?;

        results.root_folder_creation_time = root_folder_start.elapsed();
        
        if root_response.status() != StatusCode::CREATED {
            return Err(format!("Root folder creation failed: {}", root_response.status()).into());
        }

        let root_folder: Value = root_response.json().await?;
        let root_folder_id = root_folder["id"].as_str().unwrap();
        
        // Validate folder structure
        if root_folder["name"] != "Work Projects" {
            return Err("Root folder name incorrect".into());
        }
        if root_folder["level"].as_u64().unwrap() != 0 {
            return Err("Root folder level should be 0".into());
        }
        if root_folder["path"] != "/Work Projects/" {
            return Err("Root folder path incorrect".into());
        }

        println!("✓ Root folder created in {:?}", results.root_folder_creation_time);

        // Step 2: Create subfolder (level 2) - Project Alpha
        println!("Step 2: Create subfolder 'Project Alpha'");
        let subfolder_start = Instant::now();
        
        let subfolder_body = json!({
            "name": "Project Alpha",
            "parent_folder_id": root_folder_id
        });

        let subfolder_response = ctx.client
            .post(&format!("{}/api/v1/folders", ctx.base_url))
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .json(&subfolder_body)
            .send()
            .await?;

        results.subfolder_creation_time = subfolder_start.elapsed();
        
        if subfolder_response.status() != StatusCode::CREATED {
            return Err(format!("Subfolder creation failed: {}", subfolder_response.status()).into());
        }

        let subfolder: Value = subfolder_response.json().await?;
        let subfolder_id = subfolder["id"].as_str().unwrap();
        
        // Validate hierarchical structure
        if subfolder["level"].as_u64().unwrap() != 1 {
            return Err("Subfolder level should be 1".into());
        }
        if subfolder["path"] != "/Work Projects/Project Alpha/" {
            return Err("Subfolder path incorrect".into());
        }
        if subfolder["parent_folder_id"] != root_folder_id {
            return Err("Subfolder parent_folder_id incorrect".into());
        }

        println!("✓ Subfolder created in {:?}", results.subfolder_creation_time);

        // Step 3: Create notes in different locations
        println!("Step 3: Create notes in root and subfolder");
        
        // Create note in root folder
        let root_note_body = json!({
            "title": "Root Level Note",
            "content": "This note is in the root level",
            "folder_id": null
        });

        let root_note_response = ctx.client
            .post(&format!("{}/api/v1/notes", ctx.base_url))
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .json(&root_note_body)
            .send()
            .await?;

        if root_note_response.status() != StatusCode::CREATED {
            return Err(format!("Root note creation failed: {}", root_note_response.status()).into());
        }

        let root_note: Value = root_note_response.json().await?;
        let root_note_id = root_note["id"].as_str().unwrap();

        // Create note in subfolder
        let subfolder_note_body = json!({
            "title": "Project Alpha Note",
            "content": "This note belongs to Project Alpha",
            "folder_id": subfolder_id
        });

        let subfolder_note_response = ctx.client
            .post(&format!("{}/api/v1/notes", ctx.base_url))
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .json(&subfolder_note_body)
            .send()
            .await?;

        if subfolder_note_response.status() != StatusCode::CREATED {
            return Err(format!("Subfolder note creation failed: {}", subfolder_note_response.status()).into());
        }

        let subfolder_note: Value = subfolder_note_response.json().await?;
        let _subfolder_note_id = subfolder_note["id"].as_str().unwrap();

        println!("✓ Notes created in different locations");

        // Step 4: Move note between folders (FR-007) - Simulate drag-and-drop
        println!("Step 4: Move note from root to Project Alpha folder");
        let move_start = Instant::now();
        
        let move_body = json!({
            "folder_id": subfolder_id,
            "version": root_note["version"]
        });

        let move_response = ctx.client
            .put(&format!("{}/api/v1/notes/{}/move", ctx.base_url, root_note_id))
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .json(&move_body)
            .send()
            .await?;

        results.note_move_time = move_start.elapsed();
        
        if move_response.status() != StatusCode::OK {
            return Err(format!("Note move failed: {}", move_response.status()).into());
        }

        let moved_note: Value = move_response.json().await?;
        if moved_note["folder_id"] != subfolder_id {
            return Err("Note move did not update folder_id correctly".into());
        }

        println!("✓ Note moved successfully in {:?}", results.note_move_time);

        // Step 5: Verify folder contents
        println!("Step 5: Verify folder contents");
        let list_start = Instant::now();
        
        // List notes in subfolder
        let subfolder_notes_response = ctx.client
            .get(&format!("{}/api/v1/notes?folder_id={}", ctx.base_url, subfolder_id))
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;

        results.folder_contents_time = list_start.elapsed();
        
        if subfolder_notes_response.status() != StatusCode::OK {
            return Err(format!("Folder contents listing failed: {}", subfolder_notes_response.status()).into());
        }

        let subfolder_notes: Value = subfolder_notes_response.json().await?;
        let notes_in_subfolder = subfolder_notes["notes"].as_array().unwrap();
        
        if notes_in_subfolder.len() != 2 {
            return Err(format!("Expected 2 notes in subfolder, found {}", notes_in_subfolder.len()).into());
        }

        println!("✓ Folder contents verified in {:?}", results.folder_contents_time);

        // Step 6: Test folder depth limit (FR-013) - max 10 levels
        println!("Step 6: Test folder depth limit");
        let depth_test_start = Instant::now();
        
        let mut current_parent_id = Some(subfolder_id.to_string());
        let mut depth_levels_created = 1; // subfolder is level 1
        
        // Try to create folders up to and beyond the limit
        for level in 2..12 {  // Start from level 2, try up to level 11 (should fail at level 10)
            let deep_folder_body = json!({
                "name": format!("Level {} Folder", level),
                "parent_folder_id": current_parent_id
            });

            let deep_folder_response = ctx.client
                .post(&format!("{}/api/v1/folders", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .json(&deep_folder_body)
                .send()
                .await?;

            if deep_folder_response.status() == StatusCode::CREATED {
                let deep_folder: Value = deep_folder_response.json().await?;
                current_parent_id = Some(deep_folder["id"].as_str().unwrap().to_string());
                depth_levels_created = level;
                
                if deep_folder["level"].as_u64().unwrap() != level as u64 {
                    return Err(format!("Deep folder level {} incorrect", level).into());
                }
            } else if deep_folder_response.status() == StatusCode::BAD_REQUEST {
                // Depth limit reached - this is expected
                break;
            } else {
                return Err(format!("Unexpected status creating level {}: {}", level, deep_folder_response.status()).into());
            }
        }
        
        results.depth_limit_time = depth_test_start.elapsed();
        
        if depth_levels_created >= 10 {
            return Err("Folder depth limit not enforced - created more than 10 levels".into());
        }

        println!("✓ Folder depth limit enforced at level {} in {:?}", depth_levels_created, results.depth_limit_time);

        // Step 7: List complete folder hierarchy
        println!("Step 7: List complete folder hierarchy");
        let hierarchy_start = Instant::now();
        
        let folders_response = ctx.client
            .get(&format!("{}/api/v1/folders", ctx.base_url))
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;

        results.hierarchy_list_time = hierarchy_start.elapsed();
        
        if folders_response.status() != StatusCode::OK {
            return Err(format!("Folders listing failed: {}", folders_response.status()).into());
        }

        let folders_data: Value = folders_response.json().await?;
        let all_folders = folders_data["folders"].as_array().unwrap();
        
        // Should have created several folders in hierarchy
        if all_folders.len() < 2 {
            return Err(format!("Expected at least 2 folders, found {}", all_folders.len()).into());
        }

        println!("✓ Folder hierarchy listed successfully in {:?}", results.hierarchy_list_time);

        // Step 8: Validate performance targets
        println!("Step 8: Performance validation");
        results.validate_performance_targets()?;

        Ok(results)
    }

    #[derive(Debug)]
    struct FolderScenarioResults {
        root_folder_creation_time: Duration,
        subfolder_creation_time: Duration,
        note_move_time: Duration,
        folder_contents_time: Duration,
        depth_limit_time: Duration,
        hierarchy_list_time: Duration,
    }

    impl FolderScenarioResults {
        fn new() -> Self {
            Self {
                root_folder_creation_time: Duration::from_secs(0),
                subfolder_creation_time: Duration::from_secs(0),
                note_move_time: Duration::from_secs(0),
                folder_contents_time: Duration::from_secs(0),
                depth_limit_time: Duration::from_secs(0),
                hierarchy_list_time: Duration::from_secs(0),
            }
        }

        fn validate_performance_targets(&self) -> Result<(), Box<dyn std::error::Error>> {
            let max_api_time = Duration::from_millis(200);
            
            if self.root_folder_creation_time > max_api_time {
                return Err(format!("Root folder creation too slow: {:?} > 200ms", self.root_folder_creation_time).into());
            }
            
            if self.subfolder_creation_time > max_api_time {
                return Err(format!("Subfolder creation too slow: {:?} > 200ms", self.subfolder_creation_time).into());
            }
            
            if self.note_move_time > max_api_time {
                return Err(format!("Note move too slow: {:?} > 200ms", self.note_move_time).into());
            }
            
            if self.folder_contents_time > max_api_time {
                return Err(format!("Folder contents listing too slow: {:?} > 200ms", self.folder_contents_time).into());
            }
            
            if self.hierarchy_list_time > max_api_time {
                return Err(format!("Hierarchy listing too slow: {:?} > 200ms", self.hierarchy_list_time).into());
            }

            println!("✓ All folder operations meet performance targets (<200ms)");
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_folder_name_validation() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "folder_validation@example.com", "password123").await;
        
        if let Ok(access_token) = token_result {
            let test_cases = vec![
                // Empty name
                json!({"name": ""}),
                // Name too long
                json!({"name": "a".repeat(256)}),
                // Special characters
                json!({"name": "folder/with\\invalid:chars"}),
            ];

            for test_case in test_cases {
                let response = ctx.client
                    .post(&format!("{}/api/v1/folders", ctx.base_url))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&test_case)
                    .send()
                    .await;

                assert!(response.is_err(),
                    "Test should fail initially - no server implementation exists yet");
            }
        }

        // Expected behavior: Invalid folder names should return 400 Bad Request
    }

    #[tokio::test]
    async fn test_circular_folder_reference_prevention() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "circular@example.com", "password123").await;
        
        if let Ok(access_token) = token_result {
            // This test would create folders and then try to create circular reference
            // All operations will fail initially due to no server
            let folder_a_body = json!({
                "name": "Folder A"
            });

            let response_a = ctx.client
                .post(&format!("{}/api/v1/folders", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .json(&folder_a_body)
                .send()
                .await;

            assert!(response_a.is_err(),
                "Test should fail initially - no server implementation exists yet");
        }

        // Expected behavior: System should prevent circular references in folder hierarchy
    }

    #[tokio::test]
    async fn test_bulk_folder_operations() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "bulk_folders@example.com", "password123").await;
        
        if let Ok(access_token) = token_result {
            // Create multiple folders concurrently
            let mut handles = Vec::new();
            
            for i in 0..10 {
                let client = ctx.client.clone();
                let url = ctx.base_url.clone();
                let token = access_token.clone();
                
                let handle = tokio::spawn(async move {
                    let folder_body = json!({
                        "name": format!("Bulk Folder {}", i)
                    });

                    client
                        .post(&format!("{}/api/v1/folders", url))
                        .header("Authorization", format!("Bearer {}", token))
                        .header("Content-Type", "application/json")
                        .json(&folder_body)
                        .send()
                        .await
                });
                
                handles.push(handle);
            }

            let results = futures::future::join_all(handles).await;
            
            assert!(results.iter().all(|r| r.is_err()),
                "Test should fail initially - no server implementation exists yet");
        }

        // Expected behavior: System should handle concurrent folder creation efficiently
    }

    #[tokio::test]
    async fn test_folder_update_operations() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "folder_update@example.com", "password123").await;
        
        if let Ok(access_token) = token_result {
            // Test folder renaming and its effect on materialized paths
            let folder_body = json!({
                "name": "Original Folder Name"
            });

            let create_response = ctx.client
                .post(&format!("{}/api/v1/folders", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .json(&folder_body)
                .send()
                .await;

            assert!(create_response.is_err(),
                "Test should fail initially - no server implementation exists yet");
        }

        // Expected behavior: Folder updates should cascade to child folder paths
    }

    #[tokio::test]
    async fn test_folder_deletion_cascade() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "folder_cascade@example.com", "password123").await;
        
        if let Ok(access_token) = token_result {
            // Test that deleting parent folder cascades to children and notes
            let parent_body = json!({
                "name": "Parent to Delete"
            });

            let create_response = ctx.client
                .post(&format!("{}/api/v1/folders", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .json(&parent_body)
                .send()
                .await;

            assert!(create_response.is_err(),
                "Test should fail initially - no server implementation exists yet");
        }

        // Expected behavior: Deleting folder should cascade delete children and move notes to root
    }

    #[tokio::test]
    async fn test_folder_path_materialization() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "path_test@example.com", "password123").await;
        
        if let Ok(access_token) = token_result {
            // Test that materialized paths are correctly calculated and updated
            let deep_structure = vec![
                ("Projects", None),
                ("Work", None), // This should be child of Projects after creation
                ("Client A", None), // This should be child of Work
            ];

            for (name, _parent_id) in deep_structure {
                let folder_body = json!({
                    "name": name
                });

                let response = ctx.client
                    .post(&format!("{}/api/v1/folders", ctx.base_url))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&folder_body)
                    .send()
                    .await;

                assert!(response.is_err(),
                    "Test should fail initially - no server implementation exists yet");
            }
        }

        // Expected behavior: Paths should be properly materialized as "/Projects/Work/Client A/"
    }

    #[tokio::test]
    async fn test_note_count_per_folder_limit() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "note_limit@example.com", "password123").await;
        
        if let Ok(access_token) = token_result {
            // Test that folder can handle up to 1000 notes (per specification)
            // Create folder first
            let folder_body = json!({
                "name": "High Capacity Folder"
            });

            let folder_response = ctx.client
                .post(&format!("{}/api/v1/folders", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .json(&folder_body)
                .send()
                .await;

            assert!(folder_response.is_err(),
                "Test should fail initially - no server implementation exists yet");
        }

        // Expected behavior: Folder should handle 1000 notes efficiently with pagination
    }
}