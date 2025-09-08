use axum::{
    extract::{
        ws::{Message, WebSocket},
        Query, State, WebSocketUpgrade,
    },
    http::StatusCode,
    response::{IntoResponse, Response},
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::Arc,
};
use tokio::sync::{broadcast, RwLock, mpsc};
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketMessage {
    pub id: String,
    pub message_type: String,
    pub data: Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct WebSocketQuery {
    pub token: String,
}

pub type UserConnections = Arc<RwLock<HashMap<i32, Vec<String>>>>;
pub type ConnectionUsers = Arc<RwLock<HashMap<String, i32>>>;

#[derive(Clone)]
pub struct WebSocketService {
    // Map of user_id -> list of connection_ids
    user_connections: UserConnections,
    // Map of connection_id -> user_id
    connection_users: ConnectionUsers,
    // Broadcast channel for sending messages
    broadcast_tx: broadcast::Sender<(i32, WebSocketMessage)>,
    // Individual connection senders
    connection_senders: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<WebSocketMessage>>>>,
}

impl WebSocketService {
    pub fn new() -> Self {
        let (broadcast_tx, _) = broadcast::channel(1000);
        
        Self {
            user_connections: Arc::new(RwLock::new(HashMap::new())),
            connection_users: Arc::new(RwLock::new(HashMap::new())),
            broadcast_tx,
            connection_senders: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn handle_websocket_upgrade(
        &self,
        ws: WebSocketUpgrade,
        query: Query<WebSocketQuery>,
        user_id: i32,
    ) -> Response {
        let service = self.clone();
        
        ws.on_upgrade(move |socket| {
            service.handle_websocket_connection(socket, user_id, query.token.clone())
        })
    }

    async fn handle_websocket_connection(
        &self,
        socket: WebSocket,
        user_id: i32,
        _token: String,
    ) {
        let connection_id = Uuid::new_v4().to_string();
        info!("New WebSocket connection: {} for user: {}", connection_id, user_id);

        // Add connection to tracking maps
        {
            let mut user_connections = self.user_connections.write().await;
            user_connections
                .entry(user_id)
                .or_insert_with(Vec::new)
                .push(connection_id.clone());
        }

        {
            let mut connection_users = self.connection_users.write().await;
            connection_users.insert(connection_id.clone(), user_id);
        }

        // Create channel for this specific connection
        let (tx, mut rx) = mpsc::unbounded_channel();
        {
            let mut senders = self.connection_senders.write().await;
            senders.insert(connection_id.clone(), tx);
        }

        // Subscribe to broadcast channel
        let mut broadcast_rx = self.broadcast_tx.subscribe();

        // Split the WebSocket
        let (mut ws_tx, mut ws_rx) = socket.split();

        // Spawn task to handle outgoing messages to this connection
        let connection_id_clone = connection_id.clone();
        let outgoing_task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Handle direct messages to this connection
                    msg = rx.recv() => {
                        match msg {
                            Some(message) => {
                                if let Ok(json_msg) = serde_json::to_string(&message) {
                                    if ws_tx.send(Message::Text(json_msg)).await.is_err() {
                                        break;
                                    }
                                }
                            }
                            None => break,
                        }
                    }
                    // Handle broadcast messages
                    broadcast_msg = broadcast_rx.recv() => {
                        match broadcast_msg {
                            Ok((target_user_id, message)) => {
                                if target_user_id == user_id {
                                    if let Ok(json_msg) = serde_json::to_string(&message) {
                                        if ws_tx.send(Message::Text(json_msg)).await.is_err() {
                                            break;
                                        }
                                    }
                                }
                            }
                            Err(_) => break,
                        }
                    }
                }
            }
            
            info!("Outgoing message task ended for connection: {}", connection_id_clone);
        });

        // Handle incoming messages from the WebSocket
        let service_clone = self.clone();
        let connection_id_clone = connection_id.clone();
        let incoming_task = tokio::spawn(async move {
            while let Some(msg) = ws_rx.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Err(e) = service_clone.handle_incoming_message(&connection_id_clone, user_id, &text).await {
                            error!("Error handling incoming message: {}", e);
                        }
                    }
                    Ok(Message::Close(_)) => {
                        info!("WebSocket connection closed by client: {}", connection_id_clone);
                        break;
                    }
                    Ok(Message::Ping(data)) => {
                        // Respond to ping with pong
                        // Note: axum automatically handles ping/pong, but we can log it
                        info!("Received ping from connection: {}", connection_id_clone);
                    }
                    Ok(Message::Pong(_)) => {
                        // Client responded to our ping
                        info!("Received pong from connection: {}", connection_id_clone);
                    }
                    Ok(Message::Binary(_)) => {
                        warn!("Received unexpected binary message from connection: {}", connection_id_clone);
                    }
                    Err(e) => {
                        error!("WebSocket error for connection {}: {}", connection_id_clone, e);
                        break;
                    }
                }
            }
            
            info!("Incoming message task ended for connection: {}", connection_id_clone);
        });

        // Wait for either task to complete
        tokio::select! {
            _ = outgoing_task => {},
            _ = incoming_task => {},
        }

        // Cleanup connection
        self.cleanup_connection(&connection_id, user_id).await;
    }

    async fn handle_incoming_message(
        &self,
        connection_id: &str,
        user_id: i32,
        message: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let parsed_message: Value = serde_json::from_str(message)?;
        
        let message_type = parsed_message["type"]
            .as_str()
            .unwrap_or("unknown");

        match message_type {
            "ping" => {
                // Send pong response
                self.send_to_connection(
                    connection_id,
                    WebSocketMessage {
                        id: Uuid::new_v4().to_string(),
                        message_type: "pong".to_string(),
                        data: json!({}),
                        timestamp: chrono::Utc::now(),
                    },
                ).await?;
            }
            "subscribe" => {
                // Handle subscription to specific events
                let event_type = parsed_message["data"]["event"]
                    .as_str()
                    .unwrap_or("all");
                
                info!("User {} subscribed to {} events", user_id, event_type);
                
                // Send confirmation
                self.send_to_connection(
                    connection_id,
                    WebSocketMessage {
                        id: Uuid::new_v4().to_string(),
                        message_type: "subscription_confirmed".to_string(),
                        data: json!({
                            "event": event_type,
                            "status": "subscribed"
                        }),
                        timestamp: chrono::Utc::now(),
                    },
                ).await?;
            }
            "heartbeat" => {
                // Respond to heartbeat
                self.send_to_connection(
                    connection_id,
                    WebSocketMessage {
                        id: Uuid::new_v4().to_string(),
                        message_type: "heartbeat_response".to_string(),
                        data: json!({
                            "timestamp": chrono::Utc::now()
                        }),
                        timestamp: chrono::Utc::now(),
                    },
                ).await?;
            }
            _ => {
                warn!("Unknown message type: {} from user: {}", message_type, user_id);
            }
        }

        Ok(())
    }

    async fn send_to_connection(
        &self,
        connection_id: &str,
        message: WebSocketMessage,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let senders = self.connection_senders.read().await;
        if let Some(tx) = senders.get(connection_id) {
            tx.send(message)?;
        }
        Ok(())
    }

    pub async fn broadcast_to_user(&self, user_id: i32, data: Value) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let message = WebSocketMessage {
            id: Uuid::new_v4().to_string(),
            message_type: "broadcast".to_string(),
            data,
            timestamp: chrono::Utc::now(),
        };

        // Send via broadcast channel
        let _ = self.broadcast_tx.send((user_id, message));
        
        Ok(())
    }

    pub async fn send_to_user(&self, user_id: i32, message_type: String, data: Value) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let message = WebSocketMessage {
            id: Uuid::new_v4().to_string(),
            message_type,
            data,
            timestamp: chrono::Utc::now(),
        };

        let user_connections = self.user_connections.read().await;
        let senders = self.connection_senders.read().await;

        if let Some(connection_ids) = user_connections.get(&user_id) {
            for connection_id in connection_ids {
                if let Some(tx) = senders.get(connection_id) {
                    let _ = tx.send(message.clone());
                }
            }
        }

        Ok(())
    }

    pub async fn broadcast_to_all(&self, message_type: String, data: Value) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let message = WebSocketMessage {
            id: Uuid::new_v4().to_string(),
            message_type,
            data,
            timestamp: chrono::Utc::now(),
        };

        let senders = self.connection_senders.read().await;
        for tx in senders.values() {
            let _ = tx.send(message.clone());
        }

        Ok(())
    }

    pub async fn get_connected_users(&self) -> Vec<i32> {
        let user_connections = self.user_connections.read().await;
        user_connections.keys().copied().collect()
    }

    pub async fn get_user_connection_count(&self, user_id: i32) -> usize {
        let user_connections = self.user_connections.read().await;
        user_connections.get(&user_id).map_or(0, |connections| connections.len())
    }

    pub async fn is_user_connected(&self, user_id: i32) -> bool {
        let user_connections = self.user_connections.read().await;
        user_connections.contains_key(&user_id)
    }

    async fn cleanup_connection(&self, connection_id: &str, user_id: i32) {
        info!("Cleaning up connection: {} for user: {}", connection_id, user_id);

        // Remove from connection senders
        {
            let mut senders = self.connection_senders.write().await;
            senders.remove(connection_id);
        }

        // Remove from connection users
        {
            let mut connection_users = self.connection_users.write().await;
            connection_users.remove(connection_id);
        }

        // Remove from user connections
        {
            let mut user_connections = self.user_connections.write().await;
            if let Some(connections) = user_connections.get_mut(&user_id) {
                connections.retain(|id| id != connection_id);
                if connections.is_empty() {
                    user_connections.remove(&user_id);
                }
            }
        }
    }

    pub async fn send_note_update(&self, user_id: i32, note_id: i32, action: &str, data: Value) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.send_to_user(
            user_id,
            format!("note_{}", action),
            json!({
                "note_id": note_id,
                "action": action,
                "data": data
            })
        ).await
    }

    pub async fn send_folder_update(&self, user_id: i32, folder_id: i32, action: &str, data: Value) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.send_to_user(
            user_id,
            format!("folder_{}", action),
            json!({
                "folder_id": folder_id,
                "action": action,
                "data": data
            })
        ).await
    }

    pub async fn send_sync_status(&self, user_id: i32, status: &str, details: Value) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.send_to_user(
            user_id,
            "sync_status".to_string(),
            json!({
                "status": status,
                "details": details,
                "timestamp": chrono::Utc::now()
            })
        ).await
    }

    // Health check method
    pub async fn get_stats(&self) -> Value {
        let user_connections = self.user_connections.read().await;
        let connection_users = self.connection_users.read().await;
        let senders = self.connection_senders.read().await;

        json!({
            "connected_users": user_connections.len(),
            "total_connections": connection_users.len(),
            "active_senders": senders.len(),
            "users_with_multiple_connections": user_connections.values().filter(|connections| connections.len() > 1).count()
        })
    }

    // Periodic cleanup method (should be called periodically)
    pub async fn cleanup_stale_connections(&self) {
        let mut to_cleanup = Vec::new();
        
        {
            let connection_users = self.connection_users.read().await;
            let senders = self.connection_senders.read().await;
            
            // Find connections that have no corresponding sender (likely disconnected)
            for (connection_id, user_id) in connection_users.iter() {
                if !senders.contains_key(connection_id) {
                    to_cleanup.push((connection_id.clone(), *user_id));
                }
            }
        }

        // Cleanup stale connections
        for (connection_id, user_id) in to_cleanup {
            self.cleanup_connection(&connection_id, user_id).await;
        }
    }
}