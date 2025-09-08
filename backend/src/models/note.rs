use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Note {
    pub id: i32,
    pub title: String,
    pub content: String,
    pub folder_id: Option<i32>,
    pub user_id: i32,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateNoteRequest {
    pub title: String,
    pub content: String,
    pub folder_id: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateNoteRequest {
    pub title: String,
    pub content: String,
    pub folder_id: Option<i32>,
    pub version: i32,
}

#[derive(Debug, Serialize)]
pub struct NoteResponse {
    pub id: i32,
    pub title: String,
    pub content: String,
    pub folder_id: Option<i32>,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct NoteListResponse {
    pub id: i32,
    pub title: String,
    pub folder_id: Option<i32>,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub content_preview: String,
}

#[derive(Debug, Deserialize)]
pub struct SearchNotesQuery {
    pub q: String,
    pub folder_id: Option<i32>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub id: i32,
    pub title: String,
    pub content_snippet: String,
    pub folder_id: Option<i32>,
    pub folder_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub rank: Option<f32>,
}

impl From<Note> for NoteResponse {
    fn from(note: Note) -> Self {
        Self {
            id: note.id,
            title: note.title,
            content: note.content,
            folder_id: note.folder_id,
            version: note.version,
            created_at: note.created_at,
            updated_at: note.updated_at,
        }
    }
}

impl From<Note> for NoteListResponse {
    fn from(note: Note) -> Self {
        let content_preview = if note.content.len() > 200 {
            format!("{}...", &note.content[..200])
        } else {
            note.content
        };

        Self {
            id: note.id,
            title: note.title,
            folder_id: note.folder_id,
            version: note.version,
            created_at: note.created_at,
            updated_at: note.updated_at,
            content_preview,
        }
    }
}