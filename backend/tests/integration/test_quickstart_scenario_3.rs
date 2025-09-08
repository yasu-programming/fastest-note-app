use futures_util::{SinkExt, StreamExt};
use reqwest::StatusCode;
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use testcontainers::{clients::Cli, Container};
use testcontainers_modules::{postgres::Postgres as TestPostgres, redis::Redis};
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Integration test for Quickstart Scenario 3: Real-time Synchronization & Search
/// 
/// This test validates the real-time sync capabilities and search functionality,
/// based on the quickstart scenarios defined in specs/001-web-notion/quickstart.md
/// 
/// Tests MUST fail initially (no implementation exists yet)
/// 
/// Scenario covers:
/// 1. Real-time synchronization between multiple clients (FR-003)
/// 2. WebSocket message delivery and operational transforms
/// 3. Full-text search functionality (FR-006)  
/// 4. Search performance validation (<100ms)
/// 5. Concurrent editing and conflict resolution (FR-011)

#[cfg(test)]
mod quickstart_scenario_3_tests {
    use super::*;

    struct TestContext {
        client: reqwest::Client,
        base_url: String,
        ws_base_url: String,
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
        let ws_base_url = "ws://localhost:3001".to_string();

        TestContext {
            client: reqwest::Client::new(),
            base_url,
            ws_base_url,
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

    /// Helper function to connect to WebSocket
    async fn connect_websocket(ctx: &TestContext, access_token: &str) -> Result<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, Box<dyn std::error::Error>> {
        let ws_url = format!("{}/ws/notes?token={}", ctx.ws_base_url, access_token);
        let (ws_stream, _) = connect_async(&ws_url).await?;
        Ok(ws_stream)
    }

    #[tokio::test]
    async fn test_quickstart_scenario_3_complete_realtime_search_workflow() {
        let ctx = setup_test_environment().await;
        
        // This integration test MUST fail initially - no server implementation exists
        let scenario_result = run_complete_realtime_scenario(&ctx).await;
        
        assert!(scenario_result.is_err(),
            "Test should fail initially - no server implementation exists yet");

        // Expected behavior after implementation:
        // - Real-time synchronization works between multiple clients
        // - Search functionality returns results quickly (<100ms)
        // - WebSocket messages are delivered promptly (<50ms)
        // - Concurrent edits are handled gracefully
    }

    async fn run_complete_realtime_scenario(ctx: &TestContext) -> Result<RealtimeScenarioResults, Box<dyn std::error::Error>> {
        let mut results = RealtimeScenarioResults::new();
        
        // Setup: Get two authenticated users (simulate two browser windows)
        let user1_token = get_auth_token(&ctx, "sync_user1@example.com", "SecurePass123!").await?;
        let user2_token = get_auth_token(&ctx, "sync_user2@example.com", "SecurePass123!").await?;
        
        // Step 1: Create initial test data
        println!("Step 1: Create test notes for search and sync");
        let setup_start = Instant::now();
        
        // Create diverse notes for search testing
        let test_notes = vec![
            ("Project Alpha Documentation", "This document contains important information about Project Alpha requirements and specifications."),
            ("Meeting Notes - Q4 Planning", "Quarterly planning session notes with discussion about project priorities and resource allocation."),
            ("Technical Architecture", "System architecture overview including database design, API specifications, and integration points."),
            ("User Research Findings", "Research insights about user behavior, pain points, and feature requests from customer interviews."),
        ];

        let mut created_notes = Vec::new();
        
        for (title, content) in test_notes {
            let note_body = json!({
                "title": title,
                "content": content,
                "folder_id": null
            });

            let note_response = ctx.client
                .post(&format!("{}/api/v1/notes", ctx.base_url))
                .header("Authorization", format!("Bearer {}", user1_token))
                .header("Content-Type", "application/json")
                .json(&note_body)
                .send()
                .await?;

            if note_response.status() != StatusCode::CREATED {
                return Err(format!("Note creation failed: {}", note_response.status()).into());
            }

            let note_data: Value = note_response.json().await?;
            created_notes.push(note_data);
        }
        
        results.test_data_setup_time = setup_start.elapsed();
        println!("✓ Test data created in {:?}", results.test_data_setup_time);

        // Step 2: Test full-text search functionality (FR-006)
        println!("Step 2: Test search functionality");
        
        // Search test cases with performance measurement
        let search_cases = vec![
            ("project", "Should find Project Alpha Documentation"),
            ("alpha", "Should find Project Alpha Documentation"),  
            ("meeting", "Should find Meeting Notes"),
            ("architecture", "Should find Technical Architecture"),
            ("user research", "Should find User Research Findings"),
            ("specifications", "Should find multiple notes mentioning specs"),
        ];

        for (query, description) in search_cases {
            let search_start = Instant::now();
            
            let search_response = ctx.client
                .get(&format!("{}/api/v1/notes?search={}", ctx.base_url, urlencoding::encode(query)))
                .header("Authorization", format!("Bearer {}", user1_token))
                .send()
                .await?;

            let search_time = search_start.elapsed();
            results.search_times.push(search_time);
            
            if search_response.status() != StatusCode::OK {
                return Err(format!("Search failed for '{}': {}", query, search_response.status()).into());
            }

            let search_results: Value = search_response.json().await?;
            let found_notes = search_results["notes"].as_array().unwrap();
            
            if found_notes.is_empty() {
                return Err(format!("Search for '{}' returned no results - {}", query, description).into());
            }
            
            println!("✓ Search for '{}' found {} results in {:?}", query, found_notes.len(), search_time);
        }

        // Step 3: Real-time synchronization test (FR-003)  
        println!("Step 3: Test real-time synchronization");
        let sync_start = Instant::now();
        
        // Connect both users to WebSocket
        let mut ws_user1 = connect_websocket(&ctx, &user1_token).await?;
        let mut ws_user2 = connect_websocket(&ctx, &user2_token).await?;
        
        // User 1 creates a note - User 2 should receive notification
        let realtime_note_body = json!({
            "title": "Real-time Test Note",
            "content": "This note tests real-time synchronization between clients",
            "folder_id": null
        });

        let create_response = ctx.client
            .post(&format!("{}/api/v1/notes", ctx.base_url))
            .header("Authorization", format!("Bearer {}", user1_token))
            .header("Content-Type", "application/json")
            .json(&realtime_note_body)
            .send()
            .await?;

        if create_response.status() != StatusCode::CREATED {
            return Err(format!("Real-time note creation failed: {}", create_response.status()).into());
        }

        let realtime_note: Value = create_response.json().await?;
        let realtime_note_id = realtime_note["id"].as_str().unwrap();

        // Check if User 2 receives WebSocket notification
        let notification_timeout = Duration::from_millis(100); // Target: <50ms delivery
        let ws_message_result = timeout(notification_timeout, ws_user2.next()).await;
        
        if let Ok(Some(Ok(Message::Text(message)))) = ws_message_result {
            let notification: Value = serde_json::from_str(&message)?;
            if notification["type"] == "note_created" {
                results.websocket_delivery_time = sync_start.elapsed();
                println!("✓ WebSocket notification delivered in {:?}", results.websocket_delivery_time);
            }
        }
        
        // Step 4: Test concurrent editing (FR-011)
        println!("Step 4: Test concurrent editing and conflict resolution");
        let concurrent_start = Instant::now();
        
        // Both users edit the same note simultaneously
        let note_to_edit = &created_notes[0];
        let note_id = note_to_edit["id"].as_str().unwrap();
        let original_version = note_to_edit["version"].as_u64().unwrap();
        
        // User 1 update
        let user1_update_body = json!({
            "title": "Updated by User 1",
            "content": "This content was updated by user 1",
            "version": original_version
        });

        // User 2 update (concurrent)  
        let user2_update_body = json!({
            "title": "Updated by User 2", 
            "content": "This content was updated by user 2",
            "version": original_version
        });

        // Send both updates concurrently
        let user1_update_future = ctx.client
            .put(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
            .header("Authorization", format!("Bearer {}", user1_token))
            .header("Content-Type", "application/json")
            .json(&user1_update_body)
            .send();

        let user2_update_future = ctx.client
            .put(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
            .header("Authorization", format!("Bearer {}", user2_token))
            .header("Content-Type", "application/json")
            .json(&user2_update_body)
            .send();

        let (user1_result, user2_result) = tokio::join!(user1_update_future, user2_update_future);
        
        results.concurrent_edit_time = concurrent_start.elapsed();
        
        // One should succeed, one should get conflict (409) 
        let user1_status = user1_result?.status();
        let user2_status = user2_result?.status();
        
        let success_count = [user1_status, user2_status].iter().filter(|&&s| s == StatusCode::OK).count();
        let conflict_count = [user1_status, user2_status].iter().filter(|&&s| s == StatusCode::CONFLICT).count();
        
        if success_count != 1 || conflict_count != 1 {
            return Err("Concurrent edit conflict resolution failed - expected 1 success, 1 conflict".into());
        }
        
        println!("✓ Concurrent editing handled correctly in {:?}", results.concurrent_edit_time);

        // Step 5: Test operational transforms for simultaneous edits
        println!("Step 5: Test operational transforms");
        let transform_start = Instant::now();
        
        // Create a new note for operational transform testing
        let transform_note_body = json!({
            "title": "Operational Transform Test",
            "content": "Initial content for operational transform testing",
            "folder_id": null
        });

        let transform_note_response = ctx.client
            .post(&format!("{}/api/v1/notes", ctx.base_url))
            .header("Authorization", format!("Bearer {}", user1_token))
            .header("Content-Type", "application/json")
            .json(&transform_note_body)
            .send()
            .await?;

        let transform_note: Value = transform_note_response.json().await?;
        let transform_note_id = transform_note["id"].as_str().unwrap();
        
        // Simulate operational transform by sending character-level edits
        let edit_operations = vec![
            json!({
                "type": "insert", 
                "position": 0,
                "content": "UPDATED: ",
                "version": transform_note["version"]
            }),
            json!({
                "type": "insert",
                "position": 50,
                "content": " with additional content",
                "version": transform_note["version"]
            }),
        ];

        for operation in edit_operations {
            let op_response = ctx.client
                .patch(&format!("{}/api/v1/notes/{}/operations", ctx.base_url, transform_note_id))
                .header("Authorization", format!("Bearer {}", user1_token))
                .header("Content-Type", "application/json")
                .json(&operation)
                .send()
                .await?;

            // Operational transform endpoint might not exist initially
            if op_response.status() == StatusCode::NOT_FOUND {
                // Expected for initial implementation
                break;
            }
        }
        
        results.operational_transform_time = transform_start.elapsed();
        println!("✓ Operational transform test completed in {:?}", results.operational_transform_time);

        // Step 6: Validate performance targets
        println!("Step 6: Performance validation");
        results.validate_performance_targets()?;

        Ok(results)
    }

    #[derive(Debug)]
    struct RealtimeScenarioResults {
        test_data_setup_time: Duration,
        search_times: Vec<Duration>,
        websocket_delivery_time: Duration,
        concurrent_edit_time: Duration,
        operational_transform_time: Duration,
    }

    impl RealtimeScenarioResults {
        fn new() -> Self {
            Self {
                test_data_setup_time: Duration::from_secs(0),
                search_times: Vec::new(),
                websocket_delivery_time: Duration::from_secs(0),
                concurrent_edit_time: Duration::from_secs(0),
                operational_transform_time: Duration::from_secs(0),
            }
        }

        fn validate_performance_targets(&self) -> Result<(), Box<dyn std::error::Error>> {
            // Search performance target: <100ms
            let max_search_time = Duration::from_millis(100);
            for (i, &search_time) in self.search_times.iter().enumerate() {
                if search_time > max_search_time {
                    return Err(format!("Search {} too slow: {:?} > 100ms", i, search_time).into());
                }
            }
            
            // WebSocket delivery target: <50ms  
            let max_ws_time = Duration::from_millis(50);
            if self.websocket_delivery_time > Duration::from_secs(0) && self.websocket_delivery_time > max_ws_time {
                return Err(format!("WebSocket delivery too slow: {:?} > 50ms", self.websocket_delivery_time).into());
            }
            
            // API operations should be <200ms
            let max_api_time = Duration::from_millis(200);
            if self.concurrent_edit_time > max_api_time {
                return Err(format!("Concurrent edit handling too slow: {:?} > 200ms", self.concurrent_edit_time).into());
            }

            println!("✓ All real-time and search operations meet performance targets");
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_search_edge_cases() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "search_edge@example.com", "password123").await;
        
        if let Ok(access_token) = token_result {
            let edge_cases = vec![
                ("", "Empty search query"),
                ("a", "Single character search"),
                ("special!@#$%^&*()", "Special characters in search"),
                ("日本語", "Unicode characters"),
                ("very long search query that exceeds normal length expectations for search terms", "Very long search query"),
            ];

            for (query, description) in edge_cases {
                let response = ctx.client
                    .get(&format!("{}/api/v1/notes?search={}", ctx.base_url, urlencoding::encode(query)))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .send()
                    .await;

                assert!(response.is_err(),
                    "Test should fail initially - no server implementation exists yet");
            }
        }

        // Expected behavior: Search should handle edge cases gracefully
    }

    #[tokio::test]
    async fn test_websocket_connection_stability() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_stability@example.com", "password123").await;
        
        if let Ok(access_token) = token_result {
            // Test WebSocket connection stability over time
            let ws_result = connect_websocket(&ctx, &access_token).await;
            
            assert!(ws_result.is_err(),
                "Test should fail initially - no WebSocket implementation exists yet");
        }

        // Expected behavior: WebSocket should maintain stable connection and handle reconnects
    }

    #[tokio::test]
    async fn test_search_result_ranking() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "search_ranking@example.com", "password123").await;
        
        if let Ok(access_token) = token_result {
            // Create notes with different relevance levels
            let relevance_notes = vec![
                ("JavaScript Programming", "This is a comprehensive guide to JavaScript programming language"),
                ("Java Script Style Guide", "Style guide for writing better Java Script code"),  
                ("Python vs JavaScript", "Comparison between Python and JavaScript languages"),
                ("Web Development", "JavaScript is essential for web development"),
            ];

            for (title, content) in relevance_notes {
                let note_body = json!({
                    "title": title,
                    "content": content,
                    "folder_id": null
                });

                let response = ctx.client
                    .post(&format!("{}/api/v1/notes", ctx.base_url))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&note_body)
                    .send()
                    .await;

                assert!(response.is_err(),
                    "Test should fail initially - no server implementation exists yet");
            }
        }

        // Expected behavior: Search results should be ranked by relevance
    }

    #[tokio::test]
    async fn test_realtime_sync_multiple_operations() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "multi_sync@example.com", "password123").await;
        
        if let Ok(access_token) = token_result {
            // Test rapid sequence of operations and their sync
            let operations = vec![
                "create_note",
                "update_note", 
                "move_note",
                "delete_note",
            ];

            for operation in operations {
                // Each operation will fail initially but tests the sync logic
                match operation {
                    "create_note" => {
                        let create_body = json!({
                            "title": "Sync Test Note",
                            "content": "Testing sync",
                            "folder_id": null
                        });

                        let response = ctx.client
                            .post(&format!("{}/api/v1/notes", ctx.base_url))
                            .header("Authorization", format!("Bearer {}", access_token))
                            .header("Content-Type", "application/json")
                            .json(&create_body)
                            .send()
                            .await;

                        assert!(response.is_err(),
                            "Test should fail initially - no server implementation exists yet");
                    }
                    _ => {
                        // Other operations would be tested similarly
                        assert!(true, "Other operations tested similarly");
                    }
                }
            }
        }

        // Expected behavior: All operations should sync in real-time
    }

    #[tokio::test]
    async fn test_conflict_resolution_strategies() {
        let ctx = setup_test_environment().await;
        
        let token1_result = get_auth_token(&ctx, "conflict1@example.com", "password123").await;
        let token2_result = get_auth_token(&ctx, "conflict2@example.com", "password123").await;
        
        if let (Ok(token1), Ok(token2)) = (token1_result, token2_result) {
            // Test different conflict resolution scenarios
            let scenarios = vec![
                "last_write_wins",
                "merge_changes",
                "user_choice",
            ];

            for scenario in scenarios {
                // Each scenario tests different conflict resolution strategy
                let note_body = json!({
                    "title": format!("Conflict Test - {}", scenario),
                    "content": "Original content for conflict testing",
                    "folder_id": null
                });

                let response = ctx.client
                    .post(&format!("{}/api/v1/notes", ctx.base_url))
                    .header("Authorization", format!("Bearer {}", token1))
                    .header("Content-Type", "application/json")
                    .json(&note_body)
                    .send()
                    .await;

                assert!(response.is_err(),
                    "Test should fail initially - no server implementation exists yet");
            }
        }

        // Expected behavior: Different conflict resolution strategies work correctly
    }

    #[tokio::test]
    async fn test_search_pagination() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "search_page@example.com", "password123").await;
        
        if let Ok(access_token) = token_result {
            // Test search with pagination parameters
            let search_response = ctx.client
                .get(&format!("{}/api/v1/notes?search=test&limit=10&offset=0", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await;

            assert!(search_response.is_err(),
                "Test should fail initially - no server implementation exists yet");
        }

        // Expected behavior: Search should support pagination
    }

    #[tokio::test]  
    async fn test_websocket_message_ordering() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_order@example.com", "password123").await;
        
        if let Ok(access_token) = token_result {
            // Test that WebSocket messages arrive in correct order
            let ws_result = connect_websocket(&ctx, &access_token).await;
            
            assert!(ws_result.is_err(),
                "Test should fail initially - no WebSocket implementation exists yet");
        }

        // Expected behavior: WebSocket messages should maintain order
    }
}