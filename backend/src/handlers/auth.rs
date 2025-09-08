use crate::models::{CreateUserRequest, LoginRequest, RefreshTokenRequest, UserResponse};
use crate::services::AuthService;
use axum::{
    extract::{Extension, State},
    http::StatusCode,
    response::Json,
    Json as JsonExtractor,
};
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Clone)]
pub struct AuthHandlers {
    auth_service: Arc<AuthService>,
}

impl AuthHandlers {
    pub fn new(auth_service: Arc<AuthService>) -> Self {
        Self { auth_service }
    }
}

pub async fn register(
    State(auth_service): State<Arc<AuthService>>,
    JsonExtractor(request): JsonExtractor<CreateUserRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match auth_service.register(request).await {
        Ok(auth_response) => Ok(Json(json!({
            "success": true,
            "data": auth_response
        }))),
        Err(e) => {
            let error_msg = e.to_string();
            let status = if error_msg.contains("already exists") {
                StatusCode::CONFLICT
            } else if error_msg.contains("Invalid email") || error_msg.contains("Password must") {
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

pub async fn login(
    State(auth_service): State<Arc<AuthService>>,
    JsonExtractor(request): JsonExtractor<LoginRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match auth_service.login(request).await {
        Ok(auth_response) => Ok(Json(json!({
            "success": true,
            "data": auth_response
        }))),
        Err(e) => {
            let error_msg = e.to_string();
            let status = if error_msg.contains("Invalid credentials") {
                StatusCode::UNAUTHORIZED
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

pub async fn refresh_token(
    State(auth_service): State<Arc<AuthService>>,
    JsonExtractor(request): JsonExtractor<RefreshTokenRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match auth_service.refresh_token(request).await {
        Ok(auth_response) => Ok(Json(json!({
            "success": true,
            "data": auth_response
        }))),
        Err(e) => {
            let error_msg = e.to_string();
            let status = if error_msg.contains("Invalid") || error_msg.contains("expired") {
                StatusCode::UNAUTHORIZED
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

pub async fn logout(
    State(auth_service): State<Arc<AuthService>>,
    JsonExtractor(request): JsonExtractor<RefreshTokenRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match auth_service.logout(&request.refresh_token).await {
        Ok(_) => Ok(Json(json!({
            "success": true,
            "message": "Successfully logged out"
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

pub async fn get_me(
    Extension(user): Extension<crate::models::User>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let user_response = UserResponse::from(user);
    Ok(Json(json!({
        "success": true,
        "data": user_response
    })))
}

pub async fn change_password(
    State(auth_service): State<Arc<AuthService>>,
    Extension(user): Extension<crate::models::User>,
    JsonExtractor(request): JsonExtractor<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let current_password = request["current_password"]
        .as_str()
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "error": "current_password is required"
                })),
            )
        })?;

    let new_password = request["new_password"]
        .as_str()
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "error": "new_password is required"
                })),
            )
        })?;

    match auth_service.change_password(user.id, current_password, new_password).await {
        Ok(_) => Ok(Json(json!({
            "success": true,
            "message": "Password changed successfully"
        }))),
        Err(e) => {
            let error_msg = e.to_string();
            let status = if error_msg.contains("incorrect") {
                StatusCode::UNAUTHORIZED
            } else if error_msg.contains("requirements") {
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

pub async fn revoke_token(
    State(auth_service): State<Arc<AuthService>>,
    Extension(user): Extension<crate::models::User>,
    JsonExtractor(request): JsonExtractor<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let token = request["token"]
        .as_str()
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "error": "token is required"
                })),
            )
        })?;

    match auth_service.revoke_token(token).await {
        Ok(_) => Ok(Json(json!({
            "success": true,
            "message": "Token revoked successfully"
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

pub async fn get_sessions(
    State(auth_service): State<Arc<AuthService>>,
    Extension(user): Extension<crate::models::User>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match auth_service.get_user_sessions(user.id).await {
        Ok(sessions) => Ok(Json(json!({
            "success": true,
            "data": {
                "sessions": sessions,
                "count": sessions.len()
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

pub async fn revoke_all_sessions(
    State(auth_service): State<Arc<AuthService>>,
    Extension(user): Extension<crate::models::User>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match auth_service.revoke_all_sessions(user.id).await {
        Ok(_) => Ok(Json(json!({
            "success": true,
            "message": "All sessions revoked successfully"
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