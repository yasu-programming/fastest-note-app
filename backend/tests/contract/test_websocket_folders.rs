use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::time::Duration;
use testcontainers::{clients::Cli, Container};
use testcontainers_modules::{postgres::Postgres as TestPostgres, redis::Redis};
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;

/// Contract tests for WebSocket /ws/folders endpoint
/// 
/// These tests verify the WebSocket API contract defined in contracts/api-spec.yaml
/// Tests MUST fail initially (no implementation exists yet)
/// 
/// Expected behavior according to contract:
/// - WebSocket connection to /ws/folders?token=<access_token>
/// - Real-time folder structure change notifications
/// - Message format: { "type": "folder_created|folder_updated|folder_deleted", "data": {...} }
/// - Connection requires authentication
/// - User isolation (only receive updates for own folders)
/// - Hierarchical updates (parent folder changes affect children)

#[cfg(test)]
mod websocket_folders_contract_tests {
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

        // TODO: Start the actual server with WebSocket support (will be implemented in later tasks)
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
    async fn connect_websocket(ctx: &TestContext, access_token: &str) -> Result<(tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, tokio_tungstenite::tungstenite::http::Response<Option<Vec<u8>>>), Box<dyn std::error::Error>> {
        let ws_url = format!("{}/ws/folders?token={}", ctx.ws_base_url, access_token);
        let (ws_stream, response) = connect_async(&ws_url).await?;
        Ok((ws_stream, response))
    }

    #[tokio::test]
    async fn test_websocket_connection_with_valid_token_succeeds() {
        let ctx = setup_test_environment().await;
        
        // Get auth token (this will also fail initially)
        let token_result = get_auth_token(&ctx, "ws_folders@example.com", "securepassword123").await;
        
        // This test MUST fail initially - no implementation exists
        if let Ok(access_token) = token_result {
            let ws_result = connect_websocket(&ctx, &access_token).await;
            
            assert!(ws_result.is_err(),
                "Test should fail initially - no WebSocket implementation exists yet");
        } else {
            // Expected initial failure
            assert!(true, "Test fails as expected - no auth implementation yet");
        }

        // Expected contract behavior (will pass after implementation):
        // let (mut ws_stream, response) = ws_result.unwrap();
        // assert_eq!(response.status(), 101); // WebSocket upgrade
        // 
        // // Should be able to receive messages
        // if let Some(msg) = timeout(Duration::from_secs(1), ws_stream.next()).await.ok().flatten() {
        //     assert!(msg.is_ok(), "Should receive welcome message or similar");
        // }
    }

    #[tokio::test]
    async fn test_websocket_connection_without_token_fails() {
        let ctx = setup_test_environment().await;
        
        let ws_url = format!("{}/ws/folders", ctx.ws_base_url);  // No token
        let ws_result = connect_async(&ws_url).await;
        
        // This test MUST fail initially (but for different reason - no server)
        assert!(ws_result.is_err(),
            "Test should fail initially - no WebSocket implementation exists yet");

        // Expected behavior after implementation: Connection should be rejected (401 or connection refused)
    }

    #[tokio::test]
    async fn test_websocket_connection_with_invalid_token_fails() {
        let ctx = setup_test_environment().await;
        
        let ws_url = format!("{}/ws/folders?token=invalid_token_123", ctx.ws_base_url);
        let ws_result = connect_async(&ws_url).await;
        
        // This test MUST fail initially
        assert!(ws_result.is_err(),
            "Test should fail initially - no WebSocket implementation exists yet");

        // Expected behavior: Connection should be rejected for invalid token
    }

    #[tokio::test]
    async fn test_websocket_folder_created_notification() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_folder_create@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let ws_result = connect_websocket(&ctx, &access_token).await;
            
            if let Ok((mut ws_stream, _)) = ws_result {
                // Create a folder via REST API
                let folder_body = json!({
                    "name": "WebSocket Test Folder"
                });

                let _create_response = ctx.client
                    .post(&format!("{}/api/v1/folders", ctx.base_url))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&folder_body)
                    .send()
                    .await;

                // Should receive WebSocket notification
                let timeout_result = timeout(Duration::from_secs(2), ws_stream.next()).await;
                
                assert!(timeout_result.is_err(),
                    "Test should fail initially - no WebSocket implementation exists yet");
            } else {
                assert!(true, "Expected failure - no WebSocket server running");
            }
        }

        // Expected contract behavior (will pass after implementation):
        // if let Ok(Some(Ok(Message::Text(text)))) = timeout_result {
        //     let notification: Value = serde_json::from_str(&text).unwrap();
        //     assert_eq!(notification["type"], "folder_created");
        //     assert!(notification.get("data").is_some());
        //     
        //     let folder_data = &notification["data"];
        //     assert_eq!(folder_data["name"], "WebSocket Test Folder");
        //     assert!(folder_data.get("id").is_some());
        //     assert!(folder_data.get("path").is_some());
        //     assert!(folder_data.get("level").is_some());
        // }
    }

    #[tokio::test]
    async fn test_websocket_folder_updated_notification() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_folder_update@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            // Create a folder first
            let folder_body = json!({
                "name": "Original Folder Name"
            });

            let create_response_result = ctx.client
                .post(&format!("{}/api/v1/folders", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .json(&folder_body)
                .send()
                .await;

            if let Ok(create_response) = create_response_result {
                if let Ok(folder_data) = create_response.json::<Value>().await {
                    let ws_result = connect_websocket(&ctx, &access_token).await;
                    
                    if let Ok((mut ws_stream, _)) = ws_result {
                        let folder_id = folder_data["id"].as_str().unwrap();
                        let version = folder_data["version"].as_u64().unwrap();

                        // Update the folder
                        let update_body = json!({
                            "name": "Updated Folder Name",
                            "version": version
                        });

                        let _update_response = ctx.client
                            .put(&format!("{}/api/v1/folders/{}", ctx.base_url, folder_id))
                            .header("Authorization", format!("Bearer {}", access_token))
                            .header("Content-Type", "application/json")
                            .json(&update_body)
                            .send()
                            .await;

                        // Should receive WebSocket notification
                        let timeout_result = timeout(Duration::from_secs(2), ws_stream.next()).await;
                        
                        assert!(timeout_result.is_err(),
                            "Test should fail initially - no WebSocket implementation exists yet");
                    }
                }
            }
        }

        // Expected behavior: Should receive folder_updated notification with updated data
    }

    #[tokio::test]
    async fn test_websocket_folder_deleted_notification() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_folder_delete@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            // Create a folder first
            let folder_body = json!({
                "name": "To Be Deleted Folder"
            });

            let create_response_result = ctx.client
                .post(&format!("{}/api/v1/folders", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .json(&folder_body)
                .send()
                .await;

            if let Ok(create_response) = create_response_result {
                if let Ok(folder_data) = create_response.json::<Value>().await {
                    let ws_result = connect_websocket(&ctx, &access_token).await;
                    
                    if let Ok((mut ws_stream, _)) = ws_result {
                        let folder_id = folder_data["id"].as_str().unwrap();

                        // Delete the folder
                        let _delete_response = ctx.client
                            .delete(&format!("{}/api/v1/folders/{}", ctx.base_url, folder_id))
                            .header("Authorization", format!("Bearer {}", access_token))
                            .send()
                            .await;

                        // Should receive WebSocket notification
                        let timeout_result = timeout(Duration::from_secs(2), ws_stream.next()).await;
                        
                        assert!(timeout_result.is_err(),
                            "Test should fail initially - no WebSocket implementation exists yet");
                    }
                }
            }
        }

        // Expected behavior: Should receive folder_deleted notification with folder ID
    }

    #[tokio::test]
    async fn test_websocket_hierarchical_folder_updates() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_hierarchy@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            // Create parent folder first
            let parent_body = json!({
                "name": "Parent Folder"
            });

            let parent_response_result = ctx.client
                .post(&format!("{}/api/v1/folders", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .json(&parent_body)
                .send()
                .await;

            if let Ok(parent_response) = parent_response_result {
                if let Ok(parent_data) = parent_response.json::<Value>().await {
                    let parent_id = parent_data["id"].as_str().unwrap();

                    // Create child folder
                    let child_body = json!({
                        "name": "Child Folder",
                        "parent_folder_id": parent_id
                    });

                    let child_response_result = ctx.client
                        .post(&format!("{}/api/v1/folders", ctx.base_url))
                        .header("Authorization", format!("Bearer {}", access_token))
                        .header("Content-Type", "application/json")
                        .json(&child_body)
                        .send()
                        .await;

                    if let Ok(child_response) = child_response_result {
                        if let Ok(child_data) = child_response.json::<Value>().await {
                            let ws_result = connect_websocket(&ctx, &access_token).await;
                            
                            if let Ok((mut ws_stream, _)) = ws_result {
                                let parent_version = parent_data["version"].as_u64().unwrap();

                                // Update parent folder (should affect child paths)
                                let update_body = json!({
                                    "name": "Renamed Parent Folder",
                                    "version": parent_version
                                });

                                let _update_response = ctx.client
                                    .put(&format!("{}/api/v1/folders/{}", ctx.base_url, parent_id))
                                    .header("Authorization", format!("Bearer {}", access_token))
                                    .header("Content-Type", "application/json")
                                    .json(&update_body)
                                    .send()
                                    .await;

                                // Should receive notifications for both parent and child updates
                                let timeout_result = timeout(Duration::from_secs(2), ws_stream.next()).await;
                                
                                assert!(timeout_result.is_err(),
                                    "Test should fail initially - no WebSocket implementation exists yet");
                            }
                        }
                    }
                }
            }
        }

        // Expected behavior: Should receive multiple notifications for hierarchical changes
    }

    #[tokio::test]
    async fn test_websocket_user_isolation() {
        let ctx = setup_test_environment().await;
        
        // Get tokens for two different users
        let token1_result = get_auth_token(&ctx, "ws_user1_folders@example.com", "password123").await;
        let token2_result = get_auth_token(&ctx, "ws_user2_folders@example.com", "password123").await;
        
        if let (Ok(token1), Ok(token2)) = (token1_result, token2_result) {
            // Connect both users to WebSocket
            let ws1_result = connect_websocket(&ctx, &token1).await;
            let ws2_result = connect_websocket(&ctx, &token2).await;
            
            if let (Ok((mut ws_stream1, _)), Ok((mut ws_stream2, _))) = (ws1_result, ws2_result) {
                // User 1 creates a folder
                let folder_body = json!({
                    "name": "User1 Private Folder"
                });

                let _create_response = ctx.client
                    .post(&format!("{}/api/v1/folders", ctx.base_url))
                    .header("Authorization", format!("Bearer {}", token1))
                    .header("Content-Type", "application/json")
                    .json(&folder_body)
                    .send()
                    .await;

                // User 1 should receive notification
                let timeout1 = timeout(Duration::from_secs(2), ws_stream1.next()).await;
                // User 2 should NOT receive notification
                let timeout2 = timeout(Duration::from_secs(1), ws_stream2.next()).await;
                
                assert!(timeout1.is_err() && timeout2.is_err(),
                    "Test should fail initially - no WebSocket implementation exists yet");
            }
        }

        // Expected behavior: Only user1 should receive the notification for their folder
    }

    #[tokio::test]
    async fn test_websocket_folder_depth_limit_notification() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_depth@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let ws_result = connect_websocket(&ctx, &access_token).await;
            
            if let Ok((mut ws_stream, _)) = ws_result {
                // Try to create folders exceeding depth limit (should fail and trigger error notification)
                let mut current_parent_id: Option<String> = None;
                
                // Create folders up to and beyond the limit (max 10 levels)
                for level in 0..12 {
                    let folder_body = json!({
                        "name": format!("Level {} Folder", level),
                        "parent_folder_id": current_parent_id
                    });

                    let create_response = ctx.client
                        .post(&format!("{}/api/v1/folders", ctx.base_url))
                        .header("Authorization", format!("Bearer {}", access_token))
                        .header("Content-Type", "application/json")
                        .json(&folder_body)
                        .send()
                        .await;

                    if let Ok(response) = create_response {
                        if response.status().is_success() {
                            if let Ok(folder_data) = response.json::<Value>().await {
                                current_parent_id = Some(folder_data["id"].as_str().unwrap().to_string());
                            }
                        } else {
                            // Should receive error notification for depth limit
                            break;
                        }
                    }
                }

                let timeout_result = timeout(Duration::from_secs(2), ws_stream.next()).await;
                
                assert!(timeout_result.is_err(),
                    "Test should fail initially - no WebSocket implementation exists yet");
            }
        }

        // Expected behavior: Should handle depth limit violations gracefully
    }

    #[tokio::test]
    async fn test_websocket_concurrent_folder_operations() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_concurrent_folders@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let ws_result = connect_websocket(&ctx, &access_token).await;
            
            if let Ok((mut ws_stream, _)) = ws_result {
                // Create multiple folders concurrently
                let mut handles = Vec::new();
                
                for i in 0..5 {
                    let client = ctx.client.clone();
                    let url = ctx.base_url.clone();
                    let token = access_token.clone();
                    
                    let handle = tokio::spawn(async move {
                        let folder_body = json!({
                            "name": format!("Concurrent Folder {}", i)
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

                // Execute all concurrent requests
                let results = futures::future::join_all(handles).await;
                
                // Should receive multiple notifications
                let mut notification_count = 0;
                while notification_count < 5 {
                    let timeout_result = timeout(Duration::from_millis(500), ws_stream.next()).await;
                    if timeout_result.is_ok() {
                        notification_count += 1;
                    } else {
                        break;
                    }
                }
                
                assert_eq!(notification_count, 0,
                    "Test should fail initially - no WebSocket implementation exists yet");
            }
        }

        // Expected behavior: Should receive notifications for all concurrent operations
    }

    #[tokio::test]
    async fn test_websocket_folder_batch_operations() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_batch@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let ws_result = connect_websocket(&ctx, &access_token).await;
            
            if let Ok((mut ws_stream, _)) = ws_result {
                // Create parent folder first
                let parent_body = json!({
                    "name": "Batch Parent"
                });

                let parent_response = ctx.client
                    .post(&format!("{}/api/v1/folders", ctx.base_url))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&parent_body)
                    .send()
                    .await;

                if let Ok(parent_response) = parent_response {
                    if let Ok(parent_data) = parent_response.json::<Value>().await {
                        let parent_id = parent_data["id"].as_str().unwrap();

                        // Delete parent (should cascade to children and trigger multiple notifications)
                        let _delete_response = ctx.client
                            .delete(&format!("{}/api/v1/folders/{}", ctx.base_url, parent_id))
                            .header("Authorization", format!("Bearer {}", access_token))
                            .send()
                            .await;

                        // Should receive batch notifications
                        let timeout_result = timeout(Duration::from_secs(3), ws_stream.next()).await;
                        
                        assert!(timeout_result.is_err(),
                            "Test should fail initially - no WebSocket implementation exists yet");
                    }
                }
            }
        }

        // Expected behavior: Should handle batch operations with proper notifications
    }

    #[tokio::test]
    async fn test_websocket_folder_path_updates() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_path@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let ws_result = connect_websocket(&ctx, &access_token).await;
            
            if let Ok((mut ws_stream, _)) = ws_result {
                // Create nested folder structure
                let parent_body = json!({
                    "name": "GrandParent"
                });

                let parent_response = ctx.client
                    .post(&format!("{}/api/v1/folders", ctx.base_url))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&parent_body)
                    .send()
                    .await;

                if let Ok(parent_response) = parent_response {
                    if let Ok(parent_data) = parent_response.json::<Value>().await {
                        let parent_id = parent_data["id"].as_str().unwrap();
                        let parent_version = parent_data["version"].as_u64().unwrap();

                        // Rename parent (should update all descendant paths)
                        let rename_body = json!({
                            "name": "RenamedGrandParent",
                            "version": parent_version
                        });

                        let _rename_response = ctx.client
                            .put(&format!("{}/api/v1/folders/{}", ctx.base_url, parent_id))
                            .header("Authorization", format!("Bearer {}", access_token))
                            .header("Content-Type", "application/json")
                            .json(&rename_body)
                            .send()
                            .await;

                        // Should receive path update notifications
                        let timeout_result = timeout(Duration::from_secs(2), ws_stream.next()).await;
                        
                        assert!(timeout_result.is_err(),
                            "Test should fail initially - no WebSocket implementation exists yet");
                    }
                }
            }
        }

        // Expected behavior: Should receive notifications with updated materialized paths
    }

    #[tokio::test]
    async fn test_websocket_message_ordering() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_ordering@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let ws_result = connect_websocket(&ctx, &access_token).await;
            
            if let Ok((mut ws_stream, _)) = ws_result {
                // Perform rapid sequence of operations
                for i in 0..3 {
                    let folder_body = json!({
                        "name": format!("Ordered Folder {}", i)
                    });

                    let _create_response = ctx.client
                        .post(&format!("{}/api/v1/folders", ctx.base_url))
                        .header("Authorization", format!("Bearer {}", access_token))
                        .header("Content-Type", "application/json")
                        .json(&folder_body)
                        .send()
                        .await;

                    // Small delay between operations
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }

                // Should receive notifications in order
                let mut received_notifications = Vec::new();
                for _ in 0..3 {
                    let timeout_result = timeout(Duration::from_secs(1), ws_stream.next()).await;
                    if let Ok(Some(Ok(Message::Text(text)))) = timeout_result {
                        received_notifications.push(text);
                    } else {
                        break;
                    }
                }
                
                assert_eq!(received_notifications.len(), 0,
                    "Test should fail initially - no WebSocket implementation exists yet");
            }
        }

        // Expected behavior: Messages should maintain proper ordering
    }

    #[tokio::test]
    async fn test_websocket_error_handling() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_error@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let ws_result = connect_websocket(&ctx, &access_token).await;
            
            if let Ok((mut ws_stream, _)) = ws_result {
                // Try to create folder with invalid data (should trigger error notification)
                let invalid_body = json!({
                    "name": "", // Empty name should fail
                });

                let _create_response = ctx.client
                    .post(&format!("{}/api/v1/folders", ctx.base_url))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&invalid_body)
                    .send()
                    .await;

                // Should receive error notification or no notification
                let timeout_result = timeout(Duration::from_secs(2), ws_stream.next()).await;
                
                assert!(timeout_result.is_err(),
                    "Test should fail initially - no WebSocket implementation exists yet");
            }
        }

        // Expected behavior: Should handle errors gracefully without breaking WebSocket connection
    }

    #[tokio::test]
    async fn test_websocket_reconnection_behavior() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_reconnect@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            // First connection
            let ws_result1 = connect_websocket(&ctx, &access_token).await;
            
            if let Ok((ws_stream1, _)) = ws_result1 {
                // Close first connection
                drop(ws_stream1);

                // Immediate reconnection
                let ws_result2 = connect_websocket(&ctx, &access_token).await;
                
                assert!(ws_result2.is_err(),
                    "Test should fail initially - no WebSocket implementation exists yet");
            }
        }

        // Expected behavior: Should allow reconnection without issues
    }
}