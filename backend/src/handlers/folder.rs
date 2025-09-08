use crate::models::{CreateFolderRequest, UpdateFolderRequest};
use crate::services::FolderService;
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
pub struct FolderQuery {
    pub parent_id: Option<i32>,
    pub include_stats: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

#[derive(Debug, Deserialize)]
pub struct MoveFolderRequest {
    pub parent_id: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct DuplicateFolderRequest {
    pub name: Option<String>,
}

pub async fn create_folder(
    State(folder_service): State<Arc<FolderService>>,
    Extension(user): Extension<crate::models::User>,
    JsonExtractor(request): JsonExtractor<CreateFolderRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match folder_service.create_folder(request, user.id).await {
        Ok(folder) => Ok(Json(json!({
            "success": true,
            "data": folder
        }))),
        Err(e) => {
            let error_msg = e.to_string();
            let status = if error_msg.contains("empty") || 
                         error_msg.contains("exceed") || 
                         error_msg.contains("cannot contain") {
                StatusCode::BAD_REQUEST
            } else if error_msg.contains("already exists") {
                StatusCode::CONFLICT
            } else if error_msg.contains("Maximum folder depth") {
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

pub async fn get_folder(
    State(folder_service): State<Arc<FolderService>>,
    Extension(user): Extension<crate::models::User>,
    Path(folder_id): Path<i32>,
    Query(query): Query<FolderQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match folder_service.get_folder(folder_id, user.id).await {
        Ok(Some(folder)) => {
            let mut response = json!({
                "success": true,
                "data": folder
            });

            // Add stats if requested
            if query.include_stats.unwrap_or(false) {
                match folder_service.get_folder_stats(folder_id, user.id).await {
                    Ok(Some((note_count, subfolder_count))) => {
                        if let Some(data) = response["data"].as_object_mut() {
                            data.insert("note_count".to_string(), json!(note_count));
                            data.insert("subfolder_count".to_string(), json!(subfolder_count));
                        }
                    }
                    _ => {}
                }
            }

            Ok(Json(response))
        }
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "error": "Folder not found"
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

pub async fn get_user_folders(
    State(folder_service): State<Arc<FolderService>>,
    Extension(user): Extension<crate::models::User>,
    Query(query): Query<FolderQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let result = if let Some(parent_id) = query.parent_id {
        // Get children of specific folder
        folder_service.get_folder_children(Some(parent_id), user.id).await
    } else {
        // Get all user folders
        folder_service.get_user_folders(user.id).await
    };

    match result {
        Ok(folders) => Ok(Json(json!({
            "success": true,
            "data": folders,
            "count": folders.len()
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

pub async fn get_folder_tree(
    State(folder_service): State<Arc<FolderService>>,
    Extension(user): Extension<crate::models::User>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match folder_service.get_folder_tree(user.id).await {
        Ok(tree) => Ok(Json(json!({
            "success": true,
            "data": tree
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

pub async fn update_folder(
    State(folder_service): State<Arc<FolderService>>,
    Extension(user): Extension<crate::models::User>,
    Path(folder_id): Path<i32>,
    JsonExtractor(request): JsonExtractor<UpdateFolderRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match folder_service.update_folder(folder_id, request, user.id).await {
        Ok(Some(folder)) => Ok(Json(json!({
            "success": true,
            "data": folder
        }))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "error": "Folder not found"
            })),
        )),
        Err(e) => {
            let error_msg = e.to_string();
            let status = if error_msg.contains("empty") || 
                         error_msg.contains("exceed") || 
                         error_msg.contains("cannot contain") ||
                         error_msg.contains("cannot be moved to itself") ||
                         error_msg.contains("descendant") ||
                         error_msg.contains("Maximum folder depth") {
                StatusCode::BAD_REQUEST
            } else if error_msg.contains("already exists") {
                StatusCode::CONFLICT
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

pub async fn delete_folder(
    State(folder_service): State<Arc<FolderService>>,
    Extension(user): Extension<crate::models::User>,
    Path(folder_id): Path<i32>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match folder_service.delete_folder(folder_id, user.id).await {
        Ok(true) => Ok(Json(json!({
            "success": true,
            "message": "Folder deleted successfully"
        }))),
        Ok(false) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "error": "Folder not found"
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

pub async fn move_folder(
    State(folder_service): State<Arc<FolderService>>,
    Extension(user): Extension<crate::models::User>,
    Path(folder_id): Path<i32>,
    JsonExtractor(request): JsonExtractor<MoveFolderRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match folder_service.move_folder(folder_id, request.parent_id, user.id).await {
        Ok(Some(folder)) => Ok(Json(json!({
            "success": true,
            "data": folder
        }))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "error": "Folder not found"
            })),
        )),
        Err(e) => {
            let error_msg = e.to_string();
            let status = if error_msg.contains("cannot be moved to itself") ||
                         error_msg.contains("descendant") ||
                         error_msg.contains("Maximum folder depth") {
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

pub async fn search_folders(
    State(folder_service): State<Arc<FolderService>>,
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

    match folder_service.search_folders(&query.q, user.id).await {
        Ok(folders) => Ok(Json(json!({
            "success": true,
            "data": folders,
            "count": folders.len(),
            "query": query.q
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

pub async fn get_folder_breadcrumbs(
    State(folder_service): State<Arc<FolderService>>,
    Extension(user): Extension<crate::models::User>,
    Path(folder_id): Path<i32>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match folder_service.get_folder_breadcrumbs(folder_id, user.id).await {
        Ok(breadcrumbs) => Ok(Json(json!({
            "success": true,
            "data": breadcrumbs
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

pub async fn get_folder_stats(
    State(folder_service): State<Arc<FolderService>>,
    Extension(user): Extension<crate::models::User>,
    Path(folder_id): Path<i32>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match folder_service.get_folder_stats(folder_id, user.id).await {
        Ok(Some((note_count, subfolder_count))) => Ok(Json(json!({
            "success": true,
            "data": {
                "note_count": note_count,
                "subfolder_count": subfolder_count,
                "total_items": note_count + subfolder_count
            }
        }))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "error": "Folder not found"
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

pub async fn duplicate_folder(
    State(folder_service): State<Arc<FolderService>>,
    Extension(user): Extension<crate::models::User>,
    Path(folder_id): Path<i32>,
    JsonExtractor(request): JsonExtractor<DuplicateFolderRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match folder_service.duplicate_folder(folder_id, user.id, request.name).await {
        Ok(Some(folder)) => Ok(Json(json!({
            "success": true,
            "data": folder
        }))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "error": "Folder not found"
            })),
        )),
        Err(e) => {
            let error_msg = e.to_string();
            let status = if error_msg.contains("already exists") {
                StatusCode::CONFLICT
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

pub async fn validate_folder_path(
    State(folder_service): State<Arc<FolderService>>,
    Extension(user): Extension<crate::models::User>,
    Path(folder_id): Path<i32>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match folder_service.validate_folder_path(folder_id, user.id).await {
        Ok(is_valid) => Ok(Json(json!({
            "success": true,
            "data": {
                "folder_id": folder_id,
                "is_valid": is_valid
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