//! Fastest Note App Backend
//! 
//! High-performance note-taking application backend built with Rust and Axum.
//! Provides REST API and WebSocket endpoints for real-time synchronization.
//! 
//! ## Features
//! 
//! - **Blazing Fast**: Sub-200ms API response times
//! - **Real-time Sync**: WebSocket-based live collaboration
//! - **Hierarchical Organization**: Nested folder structure with 10-level depth
//! - **Full-text Search**: PostgreSQL-powered search with caching
//! - **Scalable**: Redis-backed rate limiting and session management
//! - **Security First**: JWT authentication with refresh tokens
//! - **Production Ready**: Health checks, metrics, graceful shutdown
//! 
//! ## Architecture
//! 
//! The application follows a layered architecture:
//! 
//! - **Models**: Data structures and validation
//! - **Repositories**: Data access layer with PostgreSQL
//! - **Services**: Business logic and caching
//! - **Handlers**: HTTP request/response handling
//! - **Middleware**: Cross-cutting concerns (auth, logging, CORS)
//! - **Router**: API endpoint routing and composition
//! 
//! ## Quick Start
//! 
//! ```no_run
//! use fastest_note_app_backend::{app_state::AppState, router::create_app_router};
//! 
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = fastest_note_app_backend::app_state::AppConfig::default();
//!     let app_state = std::sync::Arc::new(AppState::new(config).await?);
//!     let app = create_app_router(
//!         app_state.auth_service.clone(),
//!         app_state.folder_service.clone(),
//!         app_state.note_service.clone(),
//!         app_state.websocket_service.clone(),
//!         app_state.rate_limiter.clone(),
//!     );
//!     
//!     let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await?;
//!     axum::serve(listener, app).await?;
//!     Ok(())
//! }
//! ```

pub mod models;
pub mod repositories;
pub mod services;
pub mod handlers;
pub mod middleware;
pub mod router;
pub mod app_state;
pub mod health;
pub mod shutdown;
pub mod database;
pub mod redis;

// Re-export commonly used types
pub use app_state::{AppConfig, AppState, Environment};
pub use database::{DatabaseManager, DatabaseStats};
pub use redis::RedisManager;