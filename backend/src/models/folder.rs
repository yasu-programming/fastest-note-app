use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Folder {
    pub id: i32,
    pub name: String,
    pub parent_id: Option<i32>,
    pub path: String,
    pub depth: i32,
    pub user_id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateFolderRequest {
    pub name: String,
    pub parent_id: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateFolderRequest {
    pub name: String,
    pub parent_id: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct FolderResponse {
    pub id: i32,
    pub name: String,
    pub parent_id: Option<i32>,
    pub path: String,
    pub depth: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct FolderTreeResponse {
    pub id: i32,
    pub name: String,
    pub path: String,
    pub depth: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub children: Vec<FolderTreeResponse>,
    pub note_count: i64,
}

impl From<Folder> for FolderResponse {
    fn from(folder: Folder) -> Self {
        Self {
            id: folder.id,
            name: folder.name,
            parent_id: folder.parent_id,
            path: folder.path,
            depth: folder.depth,
            created_at: folder.created_at,
            updated_at: folder.updated_at,
        }
    }
}