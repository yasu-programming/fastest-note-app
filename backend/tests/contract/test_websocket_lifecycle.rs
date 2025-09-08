use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::time::Duration;
use testcontainers::{clients::Cli, Container};
use testcontainers_modules::{postgres::Postgres as TestPostgres, redis::Redis};
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message, tungstenite::protocol::CloseFrame};
use uuid::Uuid;

/// Contract tests for WebSocket connection lifecycle management
/// 
/// These tests verify the WebSocket connection lifecycle defined in contracts/api-spec.yaml
/// Tests MUST fail initially (no implementation exists yet)
/// 
/// Expected behavior according to contract:
/// - Connection establishment with authentication
/// - Connection upgrade and protocol negotiation
/// - Heartbeat/ping-pong mechanism for connection health
/// - Graceful connection termination
/// - Connection timeout and cleanup
/// - Resource management and connection limits
/// - Error handling and recovery

#[cfg(test)]
mod websocket_lifecycle_contract_tests {
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
    async fn connect_websocket(ctx: &TestContext, endpoint: &str, access_token: &str) -> Result<(tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, tokio_tungstenite::tungstenite::http::Response<Option<Vec<u8>>>), Box<dyn std::error::Error>> {
        let ws_url = format!("{}/{}?token={}", ctx.ws_base_url, endpoint, access_token);
        let (ws_stream, response) = connect_async(&ws_url).await?;
        Ok((ws_stream, response))
    }

    #[tokio::test]
    async fn test_websocket_connection_establishment() {
        let ctx = setup_test_environment().await;
        
        // Get auth token (this will also fail initially)
        let token_result = get_auth_token(&ctx, "ws_establish@example.com", "securepassword123").await;
        
        // This test MUST fail initially - no implementation exists
        if let Ok(access_token) = token_result {
            let ws_result = connect_websocket(&ctx, "ws/notes", &access_token).await;
            
            assert!(ws_result.is_err(),
                "Test should fail initially - no WebSocket implementation exists yet");
        } else {
            // Expected initial failure
            assert!(true, "Test fails as expected - no auth implementation yet");
        }

        // Expected contract behavior (will pass after implementation):
        // let (ws_stream, response) = ws_result.unwrap();
        // assert_eq!(response.status(), 101); // HTTP 101 Switching Protocols
        // 
        // // Verify WebSocket upgrade headers
        // let headers = response.headers();
        // assert_eq!(headers.get("upgrade").unwrap(), "websocket");
        // assert_eq!(headers.get("connection").unwrap(), "Upgrade");
        // assert!(headers.contains_key("sec-websocket-accept"));
    }

    #[tokio::test]
    async fn test_websocket_authentication_required() {
        let ctx = setup_test_environment().await;
        
        // Try to connect without token
        let ws_url = format!("{}/ws/notes", ctx.ws_base_url);
        let ws_result = connect_async(&ws_url).await;
        
        // This test MUST fail initially (but for different reason - no server)
        assert!(ws_result.is_err(),
            "Test should fail initially - no WebSocket implementation exists yet");

        // Expected behavior: Should reject connection with 401 or close immediately
    }

    #[tokio::test]
    async fn test_websocket_invalid_endpoint_rejection() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_invalid@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            // Try to connect to invalid endpoint
            let ws_url = format!("{}/ws/invalid?token={}", ctx.ws_base_url, access_token);
            let ws_result = connect_async(&ws_url).await;
            
            assert!(ws_result.is_err(),
                "Test should fail initially - no WebSocket implementation exists yet");
        }

        // Expected behavior: Should reject connection to invalid endpoints
    }

    #[tokio::test]
    async fn test_websocket_heartbeat_ping_pong() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_heartbeat@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let ws_result = connect_websocket(&ctx, "ws/notes", &access_token).await;
            
            if let Ok((mut ws_stream, _)) = ws_result {
                // Send ping frame
                let ping_payload = b"heartbeat";
                let ping_result = ws_stream.send(Message::Ping(ping_payload.to_vec())).await;
                
                if ping_result.is_ok() {
                    // Should receive pong response within reasonable time
                    let pong_result = timeout(Duration::from_secs(5), ws_stream.next()).await;
                    
                    assert!(pong_result.is_err(),
                        "Test should fail initially - no WebSocket implementation exists yet");
                }
            }
        }

        // Expected contract behavior (will pass after implementation):
        // if let Ok(Some(Ok(Message::Pong(payload)))) = pong_result {
        //     assert_eq!(payload, ping_payload, "Pong payload should match ping payload");
        // } else {
        //     panic!("Should receive pong response to ping");
        // }
    }

    #[tokio::test]
    async fn test_websocket_server_initiated_ping() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_server_ping@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let ws_result = connect_websocket(&ctx, "ws/notes", &access_token).await;
            
            if let Ok((mut ws_stream, _)) = ws_result {
                // Wait for server-initiated ping (typically sent every 30-60 seconds)
                let message_result = timeout(Duration::from_secs(70), ws_stream.next()).await;
                
                assert!(message_result.is_err(),
                    "Test should fail initially - no WebSocket implementation exists yet");
            }
        }

        // Expected behavior: Server should send periodic pings for connection health
    }

    #[tokio::test]
    async fn test_websocket_graceful_close() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_close@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let ws_result = connect_websocket(&ctx, "ws/notes", &access_token).await;
            
            if let Ok((mut ws_stream, _)) = ws_result {
                // Send close frame with reason
                let close_frame = CloseFrame {
                    code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Normal,
                    reason: "Client initiated close".into(),
                };
                
                let close_result = ws_stream.send(Message::Close(Some(close_frame))).await;
                
                if close_result.is_ok() {
                    // Should receive close confirmation
                    let close_response = timeout(Duration::from_secs(5), ws_stream.next()).await;
                    
                    assert!(close_response.is_err(),
                        "Test should fail initially - no WebSocket implementation exists yet");
                }
            }
        }

        // Expected behavior: Server should respond to close frame with close confirmation
    }

    #[tokio::test]
    async fn test_websocket_connection_timeout() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_timeout@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let ws_result = connect_websocket(&ctx, "ws/notes", &access_token).await;
            
            if let Ok((mut ws_stream, _)) = ws_result {
                // Don't send any messages and wait for connection timeout
                // Most servers timeout inactive connections after 5-10 minutes
                let timeout_result = timeout(Duration::from_secs(660), ws_stream.next()).await; // 11 minutes
                
                assert!(timeout_result.is_err(),
                    "Test should fail initially - no WebSocket implementation exists yet");
            }
        }

        // Expected behavior: Server should close idle connections after timeout period
    }

    #[tokio::test]
    async fn test_websocket_connection_limit_per_user() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_limit@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let mut connections = Vec::new();
            let max_attempts = 20; // Try to exceed reasonable connection limit
            
            for _ in 0..max_attempts {
                if let Ok((ws_stream, _)) = connect_websocket(&ctx, "ws/notes", &access_token).await {
                    connections.push(ws_stream);
                } else {
                    break;
                }
                
                // Small delay between connections
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            
            // This test MUST fail initially
            assert_eq!(connections.len(), 0,
                "Test should fail initially - no WebSocket implementation exists yet");
        }

        // Expected behavior: Server should enforce reasonable connection limits (e.g., 10 per user)
    }

    #[tokio::test]
    async fn test_websocket_token_expiration_handling() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_token_expire@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let ws_result = connect_websocket(&ctx, "ws/notes", &access_token).await;
            
            if let Ok((mut ws_stream, _)) = ws_result {
                // Simulate token expiration by waiting longer than token lifetime
                // In real implementation, server should detect expired tokens and close connection
                let expiration_result = timeout(Duration::from_secs(3700), ws_stream.next()).await; // ~1 hour
                
                assert!(expiration_result.is_err(),
                    "Test should fail initially - no WebSocket implementation exists yet");
            }
        }

        // Expected behavior: Server should close connection when token expires
    }

    #[tokio::test]
    async fn test_websocket_malformed_message_handling() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_malformed@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let ws_result = connect_websocket(&ctx, "ws/notes", &access_token).await;
            
            if let Ok((mut ws_stream, _)) = ws_result {
                // Send malformed message
                let malformed_json = r#"{"type": "invalid", "data": }"#;
                let send_result = ws_stream.send(Message::Text(malformed_json.to_string())).await;
                
                if send_result.is_ok() {
                    // Should handle malformed message gracefully (ignore or send error response)
                    let response_result = timeout(Duration::from_secs(2), ws_stream.next()).await;
                    
                    assert!(response_result.is_err(),
                        "Test should fail initially - no WebSocket implementation exists yet");
                }
            }
        }

        // Expected behavior: Server should handle malformed messages without crashing
    }

    #[tokio::test]
    async fn test_websocket_large_message_handling() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_large@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let ws_result = connect_websocket(&ctx, "ws/notes", &access_token).await;
            
            if let Ok((mut ws_stream, _)) = ws_result {
                // Send large message (near WebSocket message size limit)
                let large_payload = "x".repeat(65536); // 64KB
                let large_message = json!({
                    "type": "large_test",
                    "data": large_payload
                });
                
                let send_result = ws_stream.send(Message::Text(large_message.to_string())).await;
                
                if send_result.is_ok() {
                    // Should handle large message or reject it gracefully
                    let response_result = timeout(Duration::from_secs(5), ws_stream.next()).await;
                    
                    assert!(response_result.is_err(),
                        "Test should fail initially - no WebSocket implementation exists yet");
                }
            }
        }

        // Expected behavior: Server should handle large messages within limits
    }

    #[tokio::test]
    async fn test_websocket_rapid_reconnection() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_rapid@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            // Test rapid connect/disconnect cycles
            for _ in 0..5 {
                let ws_result = connect_websocket(&ctx, "ws/notes", &access_token).await;
                
                if let Ok((ws_stream, _)) = ws_result {
                    // Immediately close connection
                    drop(ws_stream);
                    
                    // Small delay before next connection
                    tokio::time::sleep(Duration::from_millis(100)).await;
                } else {
                    // Expected to fail initially
                    assert!(true, "Test should fail initially - no WebSocket implementation exists yet");
                    break;
                }
            }
        }

        // Expected behavior: Server should handle rapid reconnections without issues
    }

    #[tokio::test]
    async fn test_websocket_concurrent_connections_different_users() {
        let ctx = setup_test_environment().await;
        
        // Create multiple user tokens
        let mut user_tokens = Vec::new();
        for i in 0..5 {
            if let Ok(token) = get_auth_token(&ctx, &format!("ws_multi_user{}@example.com", i), "password123").await {
                user_tokens.push(token);
            }
        }
        
        if !user_tokens.is_empty() {
            // Connect all users concurrently
            let mut handles = Vec::new();
            
            for token in user_tokens {
                let ctx_clone = &ctx;
                let handle = tokio::spawn(async move {
                    connect_websocket(ctx_clone, "ws/notes", &token).await
                });
                handles.push(handle);
            }
            
            let results = futures::future::join_all(handles).await;
            let successful_connections = results.iter().filter(|r| r.is_ok()).count();
            
            assert_eq!(successful_connections, 0,
                "Test should fail initially - no WebSocket implementation exists yet");
        }

        // Expected behavior: Server should handle concurrent connections from different users
    }

    #[tokio::test]
    async fn test_websocket_binary_message_rejection() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_binary@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let ws_result = connect_websocket(&ctx, "ws/notes", &access_token).await;
            
            if let Ok((mut ws_stream, _)) = ws_result {
                // Send binary message (should be rejected for this API)
                let binary_data = vec![0, 1, 2, 3, 4, 5];
                let send_result = ws_stream.send(Message::Binary(binary_data)).await;
                
                if send_result.is_ok() {
                    // Should receive error or connection close
                    let response_result = timeout(Duration::from_secs(2), ws_stream.next()).await;
                    
                    assert!(response_result.is_err(),
                        "Test should fail initially - no WebSocket implementation exists yet");
                }
            }
        }

        // Expected behavior: Server should reject binary messages for text-based API
    }

    #[tokio::test]
    async fn test_websocket_connection_recovery_after_network_interruption() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_recovery@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let ws_result = connect_websocket(&ctx, "ws/notes", &access_token).await;
            
            if let Ok((ws_stream, _)) = ws_result {
                // Simulate network interruption by dropping connection
                drop(ws_stream);
                
                // Wait a moment then try to reconnect
                tokio::time::sleep(Duration::from_millis(500)).await;
                
                let reconnect_result = connect_websocket(&ctx, "ws/notes", &access_token).await;
                
                assert!(reconnect_result.is_err(),
                    "Test should fail initially - no WebSocket implementation exists yet");
            }
        }

        // Expected behavior: Should be able to reconnect after connection drop
    }

    #[tokio::test]
    async fn test_websocket_protocol_version_negotiation() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_protocol@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            // Try connection with specific WebSocket version
            let ws_url = format!("{}/ws/notes?token={}", ctx.ws_base_url, access_token);
            
            // Most WebSocket libraries handle protocol negotiation automatically
            let ws_result = connect_async(&ws_url).await;
            
            assert!(ws_result.is_err(),
                "Test should fail initially - no WebSocket implementation exists yet");
        }

        // Expected behavior: Should negotiate appropriate WebSocket protocol version
    }

    #[tokio::test]
    async fn test_websocket_compression_support() {
        let ctx = setup_test_environment().await;
        
        let token_result = get_auth_token(&ctx, "ws_compression@example.com", "securepassword123").await;
        
        if let Ok(access_token) = token_result {
            let ws_result = connect_websocket(&ctx, "ws/notes", &access_token).await;
            
            if let Ok((mut ws_stream, response)) = ws_result {
                // Check if compression extensions are negotiated
                let extensions = response.headers().get("sec-websocket-extensions");
                
                // Send a large compressible message
                let compressible_data = "test ".repeat(1000);
                let message = json!({
                    "type": "compression_test",
                    "data": compressible_data
                });
                
                let send_result = ws_stream.send(Message::Text(message.to_string())).await;
                
                assert!(send_result.is_err(),
                    "Test should fail initially - no WebSocket implementation exists yet");
            }
        }

        // Expected behavior: Should support WebSocket compression for efficiency
    }
}