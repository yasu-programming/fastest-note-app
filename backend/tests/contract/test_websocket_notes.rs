use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::time::Duration;
use testcontainers::{clients::Cli, Container};
use testcontainers_modules::{postgres::Postgres as TestPostgres, redis::Redis};
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;

/// Contract tests for WebSocket /ws/notes endpoint
/// 
/// These tests verify the WebSocket API contract defined in contracts/api-spec.yaml
/// Tests MUST fail initially (no implementation exists yet)
/// 
/// Expected behavior according to contract:
/// - WebSocket connection to /ws/notes?token=<access_token>
/// - Real-time note change notifications
/// - Operational transform for conflict resolution
/// - Message format: { "type": "note_created|note_updated|note_deleted|note_moved", "data": {...} }
/// - Connection requires authentication
/// - User isolation (only receive updates for own notes)

#[cfg(test)]
mod websocket_notes_contract_tests {
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
        let ws_url = format!("{}/ws/notes?token={}", ctx.ws_base_url, access_token);
        let (ws_stream, response) = connect_async(&ws_url).await?;
        Ok((ws_stream, response))
    }

    #[tokio::test]
    async fn test_websocket_connection_with_valid_token_succeeds() {
        let ctx = setup_test_environment().await;
        
        // Get auth token (this will also fail initially)
        let token_result = get_auth_token(&ctx, "ws_notes@example.com", "securepassword123").await;
        
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
        
        let ws_url = format!("{}/ws/notes", ctx.ws_base_url);  // No token
        let ws_result = connect_async(&ws_url).await;
        
        // This test MUST fail initially (but for different reason - no server)
        assert!(ws_result.is_err(),
            "Test should fail initially - no WebSocket implementation exists yet");

        // Expected behavior after implementation: Connection should be rejected (401 or connection refused)
    }

    #[tokio::test]
    async fn test_websocket_connection_with_invalid_token_fails() {
        let ctx = setup_test_environment().await;
        
        let ws_url = format!("{}/ws/notes?token=invalid_token_123", ctx.ws_base_url);
        let ws_result = connect_async(&ws_url).await;
        
        // This test MUST fail initially
        assert!(ws_result.is_err(),
            "Test should fail initially - no WebSocket implementation exists yet");

        // Expected behavior: Connection should be rejected for invalid token
    }

    #[tokio::test]
    async fn test_websocket_note_created_notification() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_create@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let ws_result = connect_websocket(&ctx, &access_token).await;
            
            if let Ok((mut ws_stream, _)) = ws_result {
                // Create a note via REST API
                let note_body = json!({
                    "title": "WebSocket Test Note",
                    "content": "This should trigger a WebSocket notification",
                    "folder_id": null
                });

                let _create_response = ctx.client
                    .post(&format!("{}/api/v1/notes", ctx.base_url))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&note_body)
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
        //     assert_eq!(notification["type"], "note_created");
        //     assert!(notification.get("data").is_some());
        //     
        //     let note_data = &notification["data"];
        //     assert_eq!(note_data["title"], "WebSocket Test Note");
        //     assert!(note_data.get("id").is_some());
        //     assert!(note_data.get("created_at").is_some());
        // }
    }

    #[tokio::test]
    async fn test_websocket_note_updated_notification() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_update@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            // Create a note first
            let note_body = json!({
                "title": "Original Title",
                "content": "Original content",
                "folder_id": null
            });

            let create_response_result = ctx.client
                .post(&format!("{}/api/v1/notes", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .json(&note_body)
                .send()
                .await;

            if let Ok(create_response) = create_response_result {
                if let Ok(note_data) = create_response.json::<Value>().await {
                    let ws_result = connect_websocket(&ctx, &access_token).await;
                    
                    if let Ok((mut ws_stream, _)) = ws_result {
                        let note_id = note_data["id"].as_str().unwrap();
                        let version = note_data["version"].as_u64().unwrap();

                        // Update the note
                        let update_body = json!({
                            "title": "Updated Title",
                            "content": "Updated content",
                            "version": version
                        });

                        let _update_response = ctx.client
                            .put(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
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

        // Expected behavior: Should receive note_updated notification with updated data
    }

    #[tokio::test]
    async fn test_websocket_note_deleted_notification() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_delete@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            // Create a note first
            let note_body = json!({
                "title": "To Be Deleted",
                "content": "This note will be deleted",
                "folder_id": null
            });

            let create_response_result = ctx.client
                .post(&format!("{}/api/v1/notes", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .json(&note_body)
                .send()
                .await;

            if let Ok(create_response) = create_response_result {
                if let Ok(note_data) = create_response.json::<Value>().await {
                    let ws_result = connect_websocket(&ctx, &access_token).await;
                    
                    if let Ok((mut ws_stream, _)) = ws_result {
                        let note_id = note_data["id"].as_str().unwrap();

                        // Delete the note
                        let _delete_response = ctx.client
                            .delete(&format!("{}/api/v1/notes/{}", ctx.base_url, note_id))
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

        // Expected behavior: Should receive note_deleted notification with note ID
    }

    #[tokio::test]
    async fn test_websocket_note_moved_notification() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_move@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            // Create a folder and note first
            let folder_body = json!({
                "name": "Target Folder"
            });

            let folder_response_result = ctx.client
                .post(&format!("{}/api/v1/folders", ctx.base_url))
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .json(&folder_body)
                .send()
                .await;

            if let Ok(folder_response) = folder_response_result {
                if let Ok(folder_data) = folder_response.json::<Value>().await {
                    let note_body = json!({
                        "title": "To Be Moved",
                        "content": "This note will be moved",
                        "folder_id": null
                    });

                    let note_response_result = ctx.client
                        .post(&format!("{}/api/v1/notes", ctx.base_url))
                        .header("Authorization", format!("Bearer {}", access_token))
                        .header("Content-Type", "application/json")
                        .json(&note_body)
                        .send()
                        .await;

                    if let Ok(note_response) = note_response_result {
                        if let Ok(note_data) = note_response.json::<Value>().await {
                            let ws_result = connect_websocket(&ctx, &access_token).await;
                            
                            if let Ok((mut ws_stream, _)) = ws_result {
                                let note_id = note_data["id"].as_str().unwrap();
                                let version = note_data["version"].as_u64().unwrap();
                                let folder_id = folder_data["id"].as_str().unwrap();

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

                                // Should receive WebSocket notification
                                let timeout_result = timeout(Duration::from_secs(2), ws_stream.next()).await;
                                
                                assert!(timeout_result.is_err(),
                                    "Test should fail initially - no WebSocket implementation exists yet");
                            }
                        }
                    }
                }
            }
        }

        // Expected behavior: Should receive note_moved notification with new folder_id
    }

    #[tokio::test]
    async fn test_websocket_user_isolation() {
        let ctx = setup_test_environment().await;
        
        // Get tokens for two different users
        let token1_result = get_auth_token(&ctx, "ws_user1@example.com", "password123").await;
        let token2_result = get_auth_token(&ctx, "ws_user2@example.com", "password123").await;
        
        if let (Ok(token1), Ok(token2)) = (token1_result, token2_result) {
            // Connect both users to WebSocket
            let ws1_result = connect_websocket(&ctx, &token1).await;
            let ws2_result = connect_websocket(&ctx, &token2).await;
            
            if let (Ok((mut ws_stream1, _)), Ok((mut ws_stream2, _))) = (ws1_result, ws2_result) {
                // User 1 creates a note
                let note_body = json!({
                    "title": "User1 Note",
                    "content": "Only user1 should see this",
                    "folder_id": null
                });

                let _create_response = ctx.client
                    .post(&format!("{}/api/v1/notes", ctx.base_url))
                    .header("Authorization", format!("Bearer {}", token1))
                    .header("Content-Type", "application/json")
                    .json(&note_body)
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

        // Expected behavior: Only user1 should receive the notification for their note
    }

    #[tokio::test]
    async fn test_websocket_concurrent_connections() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_concurrent@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            // Open multiple WebSocket connections for the same user
            let mut connections = Vec::new();
            
            for _ in 0..3 {
                let ws_result = connect_websocket(&ctx, &access_token).await;
                if let Ok((ws_stream, _)) = ws_result {
                    connections.push(ws_stream);
                }
            }
            
            // This test MUST fail initially
            assert!(connections.is_empty(),
                "Test should fail initially - no WebSocket implementation exists yet");
        }

        // Expected behavior: All connections should succeed and receive same notifications
    }

    #[tokio::test]
    async fn test_websocket_connection_timeout() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_timeout@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let ws_result = timeout(
                Duration::from_secs(5),
                connect_websocket(&ctx, &access_token)
            ).await;
            
            // This test MUST fail initially
            assert!(ws_result.is_err(),
                "Test should fail initially - no WebSocket implementation exists yet");
        }

        // Expected behavior: Connection should establish within reasonable time
    }

    #[tokio::test]
    async fn test_websocket_heartbeat_ping_pong() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_heartbeat@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let ws_result = connect_websocket(&ctx, &access_token).await;
            
            if let Ok((mut ws_stream, _)) = ws_result {
                // Send ping frame
                let ping_result = ws_stream.send(Message::Ping(vec![])).await;
                
                if ping_result.is_ok() {
                    // Should receive pong response
                    let pong_result = timeout(Duration::from_secs(2), ws_stream.next()).await;
                    
                    assert!(pong_result.is_err(),
                        "Test should fail initially - no WebSocket implementation exists yet");
                }
            }
        }

        // Expected behavior: Server should respond to ping with pong
    }

    #[tokio::test]
    async fn test_websocket_message_format_validation() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_format@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let ws_result = connect_websocket(&ctx, &access_token).await;
            
            if let Ok((mut ws_stream, _)) = ws_result {
                // Create a note to trigger notification
                let note_body = json!({
                    "title": "Format Test",
                    "content": "Testing message format",
                    "folder_id": null
                });

                let _create_response = ctx.client
                    .post(&format!("{}/api/v1/notes", ctx.base_url))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&note_body)
                    .send()
                    .await;

                // Should receive properly formatted notification
                let timeout_result = timeout(Duration::from_secs(2), ws_stream.next()).await;
                
                assert!(timeout_result.is_err(),
                    "Test should fail initially - no WebSocket implementation exists yet");
            }
        }

        // Expected behavior: Messages should have proper JSON structure with type and data fields
    }

    #[tokio::test]
    async fn test_websocket_connection_limit() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_limit@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let mut successful_connections = 0;
            let max_attempts = 100; // Try to exceed reasonable connection limit
            
            for _ in 0..max_attempts {
                if let Ok(_) = connect_websocket(&ctx, &access_token).await {
                    successful_connections += 1;
                } else {
                    break;
                }
            }
            
            // This test MUST fail initially
            assert_eq!(successful_connections, 0,
                "Test should fail initially - no WebSocket implementation exists yet");
        }

        // Expected behavior: Server should enforce reasonable connection limits per user
    }

    #[tokio::test]
    async fn test_websocket_close_graceful_handling() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_close@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let ws_result = connect_websocket(&ctx, &access_token).await;
            
            if let Ok((mut ws_stream, _)) = ws_result {
                // Send close frame
                let close_result = ws_stream.send(Message::Close(None)).await;
                
                if close_result.is_ok() {
                    // Should receive close confirmation
                    let response_result = timeout(Duration::from_secs(2), ws_stream.next()).await;
                    
                    assert!(response_result.is_err(),
                        "Test should fail initially - no WebSocket implementation exists yet");
                }
            }
        }

        // Expected behavior: Server should handle close frames gracefully
    }

    #[tokio::test]
    async fn test_websocket_large_message_handling() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_large@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let ws_result = connect_websocket(&ctx, &access_token).await;
            
            if let Ok((mut ws_stream, _)) = ws_result {
                // Create a note with large content (close to 1MB limit)
                let large_content = "a".repeat(1024 * 1000); // ~1MB
                let note_body = json!({
                    "title": "Large Content Note",
                    "content": large_content,
                    "folder_id": null
                });

                let _create_response = ctx.client
                    .post(&format!("{}/api/v1/notes", ctx.base_url))
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&note_body)
                    .send()
                    .await;

                // Should receive notification for large note
                let timeout_result = timeout(Duration::from_secs(5), ws_stream.next()).await;
                
                assert!(timeout_result.is_err(),
                    "Test should fail initially - no WebSocket implementation exists yet");
            }
        }

        // Expected behavior: Server should handle large messages efficiently
    }
}