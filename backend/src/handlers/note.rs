use crate::models::{CreateNoteRequest, UpdateNoteRequest, SearchNotesQuery};
use crate::services::NoteService;
use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::Json,
    Json as JsonExtractor,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct NoteQuery {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
    pub folder_id: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub folder_id: Option<i32>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct MoveNotesRequest {
    pub note_ids: Vec<i32>,
    pub folder_id: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct BatchDeleteRequest {
    pub note_ids: Vec<i32>,
}

#[derive(Debug, Deserialize)]
pub struct AnalyticsQuery {
    pub days: Option<i32>,
}

pub async fn create_note(
    State(note_service): State<Arc<NoteService>>,
    Extension(user): Extension<crate::models::User>,
    JsonExtractor(request): JsonExtractor<CreateNoteRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match note_service.create_note(request, user.id).await {
        Ok(note) => Ok(Json(json!({
            "success": true,
            "data": note
        }))),
        Err(e) => {
            let error_msg = e.to_string();
            let status = if error_msg.contains("empty") || 
                         error_msg.contains("exceed") || 
                         error_msg.contains("1MB limit") ||
                         error_msg.contains("255 characters") {
                StatusCode::BAD_REQUEST
            } else if error_msg.contains("not found") || error_msg.contains("access denied") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };

            Err((
                status,
                Json(json!({
                    "success": false,
                    "error": error_msg
                })),
            ))
        }
    }
}

pub async fn get_note(
    State(note_service): State<Arc<NoteService>>,
    Extension(user): Extension<crate::models::User>,
    Path(note_id): Path<i32>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match note_service.get_note(note_id, user.id).await {
        Ok(Some(note)) => Ok(Json(json!({
            "success": true,
            "data": note
        }))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "error": "Note not found"
            })),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "error": e.to_string()
            })),
        )),
    }
}

pub async fn get_user_notes(
    State(note_service): State<Arc<NoteService>>,
    Extension(user): Extension<crate::models::User>,
    Query(query): Query<NoteQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let result = if let Some(folder_id) = query.folder_id {
        // Get notes from specific folder
        note_service.get_folder_notes(folder_id, user.id, query.limit, query.offset).await
    } else {
        // Get all user notes
        note_service.get_user_notes(user.id, query.limit, query.offset).await
    };

    match result {
        Ok(notes) => Ok(Json(json!({
            "success": true,
            "data": notes,
            "count": notes.len(),
            "limit": query.limit,
            "offset": query.offset
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "error": e.to_string()
            })),
        )),
    }
}

pub async fn update_note(
    State(note_service): State<Arc<NoteService>>,
    Extension(user): Extension<crate::models::User>,
    Path(note_id): Path<i32>,
    JsonExtractor(request): JsonExtractor<UpdateNoteRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match note_service.update_note(note_id, request, user.id).await {
        Ok(Some(note)) => Ok(Json(json!({
            "success": true,
            "data": note
        }))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "error": "Note not found"
            })),
        )),
        Err(e) => {
            let error_msg = e.to_string();
            let status = if error_msg.contains("empty") || 
                         error_msg.contains("exceed") || 
                         error_msg.contains("1MB limit") ||
                         error_msg.contains("255 characters") {
                StatusCode::BAD_REQUEST
            } else if error_msg.contains("Optimistic locking failed") {
                StatusCode::CONFLICT
            } else if error_msg.contains("not found") || error_msg.contains("access denied") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };

            Err((
                status,
                Json(json!({
                    "success": false,
                    "error": error_msg
                })),
            ))
        }
    }
}

pub async fn delete_note(
    State(note_service): State<Arc<NoteService>>,
    Extension(user): Extension<crate::models::User>,
    Path(note_id): Path<i32>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match note_service.delete_note(note_id, user.id).await {
        Ok(true) => Ok(Json(json!({
            "success": true,
            "message": "Note deleted successfully"
        }))),
        Ok(false) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "error": "Note not found"
            })),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "error": e.to_string()
            })),
        )),
    }
}

pub async fn search_notes(
    State(note_service): State<Arc<NoteService>>,
    Extension(user): Extension<crate::models::User>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if query.q.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "error": "Search query cannot be empty"
            })),
        ));
    }

    let search_query = SearchNotesQuery {
        q: query.q.clone(),
        folder_id: query.folder_id,
        limit: query.limit,
        offset: query.offset,
    };

    match note_service.search_notes(search_query, user.id).await {
        Ok(results) => Ok(Json(json!({
            "success": true,
            "data": results,
            "count": results.len(),
            "query": query.q,
            "folder_id": query.folder_id
        }))),
        Err(e) => {
            let error_msg = e.to_string();
            let status = if error_msg.contains("too long") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };

            Err((
                status,
                Json(json!({
                    "success": false,
                    "error": error_msg
                })),
            ))
        }
    }
}

pub async fn get_recent_notes(
    State(note_service): State<Arc<NoteService>>,
    Extension(user): Extension<crate::models::User>,
    Query(query): Query<NoteQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match note_service.get_recent_notes(user.id, query.limit).await {
        Ok(notes) => Ok(Json(json!({
            "success": true,
            "data": notes,
            "count": notes.len()
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "error": e.to_string()
            })),
        )),
    }
}

pub async fn move_notes(
    State(note_service): State<Arc<NoteService>>,
    Extension(user): Extension<crate::models::User>,
    JsonExtractor(request): JsonExtractor<MoveNotesRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if request.note_ids.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "error": "No note IDs provided"
            })),
        ));
    }

    match note_service.move_notes_to_folder(request.note_ids.clone(), request.folder_id, user.id).await {
        Ok(moved_count) => Ok(Json(json!({
            "success": true,
            "data": {
                "moved_count": moved_count,
                "requested_count": request.note_ids.len(),
                "folder_id": request.folder_id
            }
        }))),
        Err(e) => {
            let error_msg = e.to_string();
            let status = if error_msg.contains("more than 100") {
                StatusCode::BAD_REQUEST
            } else if error_msg.contains("not found") || error_msg.contains("access denied") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };

            Err((
                status,
                Json(json!({
                    "success": false,
                    "error": error_msg
                })),
            ))
        }
    }
}

pub async fn duplicate_note(
    State(note_service): State<Arc<NoteService>>,
    Extension(user): Extension<crate::models::User>,
    Path(note_id): Path<i32>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match note_service.duplicate_note(note_id, user.id).await {
        Ok(Some(note)) => Ok(Json(json!({
            "success": true,
            "data": note
        }))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "error": "Note not found"
            })),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "error": e.to_string()
            })),
        )),
    }
}

pub async fn batch_delete_notes(
    State(note_service): State<Arc<NoteService>>,
    Extension(user): Extension<crate::models::User>,
    JsonExtractor(request): JsonExtractor<BatchDeleteRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if request.note_ids.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "error": "No note IDs provided"
            })),
        ));
    }

    match note_service.batch_delete_notes(request.note_ids.clone(), user.id).await {
        Ok(deleted_count) => Ok(Json(json!({
            "success": true,
            "data": {
                "deleted_count": deleted_count,
                "requested_count": request.note_ids.len()
            }
        }))),
        Err(e) => {
            let error_msg = e.to_string();
            let status = if error_msg.contains("more than 100") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };

            Err((
                status,
                Json(json!({
                    "success": false,
                    "error": error_msg
                })),
            ))
        }
    }
}

pub async fn get_note_count(
    State(note_service): State<Arc<NoteService>>,
    Extension(user): Extension<crate::models::User>,
    Query(query): Query<NoteQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match note_service.get_note_count(user.id, query.folder_id).await {
        Ok(count) => Ok(Json(json!({
            "success": true,
            "data": {
                "count": count,
                "folder_id": query.folder_id
            }
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "error": e.to_string()
            })),
        )),
    }
}

pub async fn get_note_statistics(
    State(note_service): State<Arc<NoteService>>,
    Extension(user): Extension<crate::models::User>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match note_service.get_note_statistics(user.id).await {
        Ok(stats) => Ok(Json(json!({
            "success": true,
            "data": stats
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "error": e.to_string()
            })),
        )),
    }
}

pub async fn get_note_analytics(
    State(note_service): State<Arc<NoteService>>,
    Extension(user): Extension<crate::models::User>,
    Query(query): Query<AnalyticsQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let days = query.days.unwrap_or(30).clamp(1, 365);

    match note_service.get_note_analytics(user.id, days).await {
        Ok(analytics) => Ok(Json(json!({
            "success": true,
            "data": analytics
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "error": e.to_string()
            })),
        )),
    }
}