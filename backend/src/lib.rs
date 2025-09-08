//! Fast Note-Taking Backend
//! 
//! High-performance note-taking application backend built with Rust and Axum.
//! Provides REST API and WebSocket endpoints for real-time synchronization.

pub mod database;
pub mod redis;

// Re-export commonly used types
pub use database::{DatabaseManager, DatabaseStats};
pub use redis::RedisManager;