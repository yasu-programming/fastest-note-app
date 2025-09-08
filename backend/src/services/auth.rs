use crate::models::{User, CreateUserRequest, LoginRequest, AuthResponse, UserResponse, RefreshTokenRequest};
use crate::repositories::UserRepository;
use anyhow::{Result, anyhow};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: i64,
    pub iat: i64,
    pub user_id: i32,
    pub email: String,
}

#[derive(Clone)]
pub struct AuthService {
    user_repo: UserRepository,
    redis_pool: deadpool_redis::Pool,
    jwt_secret: String,
}

impl AuthService {
    pub fn new(user_repo: UserRepository, redis_pool: deadpool_redis::Pool, jwt_secret: String) -> Self {
        Self {
            user_repo,
            redis_pool,
            jwt_secret,
        }
    }

    pub async fn register(&self, request: CreateUserRequest) -> Result<AuthResponse> {
        // Validate email format
        if !self.is_valid_email(&request.email) {
            return Err(anyhow!("Invalid email format"));
        }

        // Validate password strength
        if !self.is_valid_password(&request.password) {
            return Err(anyhow!("Password must be at least 8 characters long and contain at least one uppercase letter, one lowercase letter, one number, and one special character"));
        }

        // Check if user already exists
        if self.user_repo.email_exists(&request.email).await? {
            return Err(anyhow!("User with this email already exists"));
        }

        // Create user
        let user = self.user_repo.create_user(request).await?;

        // Generate tokens
        let access_token = self.generate_access_token(&user)?;
        let refresh_token = self.generate_refresh_token(&user).await?;

        Ok(AuthResponse {
            access_token,
            refresh_token,
            user: user.into(),
        })
    }

    pub async fn login(&self, request: LoginRequest) -> Result<AuthResponse> {
        // Find user by email
        let user = self.user_repo.find_by_email(&request.email).await?
            .ok_or_else(|| anyhow!("Invalid credentials"))?;

        // Verify password
        if !self.user_repo.verify_password(&user, &request.password).await? {
            return Err(anyhow!("Invalid credentials"));
        }

        // Generate tokens
        let access_token = self.generate_access_token(&user)?;
        let refresh_token = self.generate_refresh_token(&user).await?;

        Ok(AuthResponse {
            access_token,
            refresh_token,
            user: user.into(),
        })
    }

    pub async fn refresh_token(&self, request: RefreshTokenRequest) -> Result<AuthResponse> {
        // Validate refresh token
        let user_id = self.validate_refresh_token(&request.refresh_token).await?;
        
        // Get user
        let user = self.user_repo.find_by_id(user_id).await?
            .ok_or_else(|| anyhow!("User not found"))?;

        // Invalidate old refresh token
        self.invalidate_refresh_token(&request.refresh_token).await?;

        // Generate new tokens
        let access_token = self.generate_access_token(&user)?;
        let refresh_token = self.generate_refresh_token(&user).await?;

        Ok(AuthResponse {
            access_token,
            refresh_token,
            user: user.into(),
        })
    }

    pub async fn logout(&self, refresh_token: &str) -> Result<()> {
        self.invalidate_refresh_token(refresh_token).await
    }

    pub async fn validate_access_token(&self, token: &str) -> Result<Claims> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_ref()),
            &Validation::default(),
        )
        .map_err(|_| anyhow!("Invalid token"))?;

        // Check if token is blacklisted
        let mut conn = self.redis_pool.get().await?;
        let is_blacklisted: bool = conn.exists(format!("blacklist:{}", token)).await?;
        
        if is_blacklisted {
            return Err(anyhow!("Token has been revoked"));
        }

        Ok(token_data.claims)
    }

    pub async fn get_user_by_token(&self, token: &str) -> Result<User> {
        let claims = self.validate_access_token(token).await?;
        
        let user = self.user_repo.find_by_id(claims.user_id).await?
            .ok_or_else(|| anyhow!("User not found"))?;

        Ok(user)
    }

    pub async fn revoke_token(&self, token: &str) -> Result<()> {
        let claims = self.validate_access_token(token).await?;
        
        // Add token to blacklist
        let mut conn = self.redis_pool.get().await?;
        let expires_at = claims.exp - Utc::now().timestamp();
        
        if expires_at > 0 {
            conn.setex(
                format!("blacklist:{}", token),
                expires_at as usize,
                "revoked"
            ).await?;
        }

        Ok(())
    }

    fn generate_access_token(&self, user: &User) -> Result<String> {
        let now = Utc::now();
        let exp = now + Duration::hours(1); // 1 hour expiry

        let claims = Claims {
            sub: user.id.to_string(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            user_id: user.id,
            email: user.email.clone(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_ref()),
        )?;

        Ok(token)
    }

    async fn generate_refresh_token(&self, user: &User) -> Result<String> {
        let token = Uuid::new_v4().to_string();
        let expires_in_days = 30;
        
        // Store refresh token in Redis
        let mut conn = self.redis_pool.get().await?;
        conn.setex(
            format!("refresh_token:{}", token),
            expires_in_days * 24 * 60 * 60, // 30 days in seconds
            user.id
        ).await?;

        Ok(token)
    }

    async fn validate_refresh_token(&self, token: &str) -> Result<i32> {
        let mut conn = self.redis_pool.get().await?;
        let user_id: Option<i32> = conn.get(format!("refresh_token:{}", token)).await?;
        
        user_id.ok_or_else(|| anyhow!("Invalid or expired refresh token"))
    }

    async fn invalidate_refresh_token(&self, token: &str) -> Result<()> {
        let mut conn = self.redis_pool.get().await?;
        conn.del(format!("refresh_token:{}", token)).await?;
        Ok(())
    }

    fn is_valid_email(&self, email: &str) -> bool {
        let email_regex = regex::Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
        email_regex.is_match(email) && email.len() <= 255
    }

    fn is_valid_password(&self, password: &str) -> bool {
        if password.len() < 8 {
            return false;
        }

        let has_upper = password.chars().any(|c| c.is_uppercase());
        let has_lower = password.chars().any(|c| c.is_lowercase());
        let has_digit = password.chars().any(|c| c.is_numeric());
        let has_special = password.chars().any(|c| "!@#$%^&*(),.?\":{}|<>".contains(c));

        has_upper && has_lower && has_digit && has_special
    }

    pub async fn change_password(&self, user_id: i32, current_password: &str, new_password: &str) -> Result<()> {
        // Get current user
        let user = self.user_repo.find_by_id(user_id).await?
            .ok_or_else(|| anyhow!("User not found"))?;

        // Verify current password
        if !self.user_repo.verify_password(&user, current_password).await? {
            return Err(anyhow!("Current password is incorrect"));
        }

        // Validate new password
        if !self.is_valid_password(new_password) {
            return Err(anyhow!("New password does not meet requirements"));
        }

        // Update password
        self.user_repo.update_password(user_id, new_password).await?;

        Ok(())
    }

    pub async fn get_user_sessions(&self, user_id: i32) -> Result<Vec<String>> {
        let mut conn = self.redis_pool.get().await?;
        let pattern = format!("refresh_token:*");
        let keys: Vec<String> = conn.keys(pattern).await?;
        
        let mut user_sessions = Vec::new();
        for key in keys {
            let stored_user_id: Option<i32> = conn.get(&key).await?;
            if let Some(stored_id) = stored_user_id {
                if stored_id == user_id {
                    user_sessions.push(key.replace("refresh_token:", ""));
                }
            }
        }

        Ok(user_sessions)
    }

    pub async fn revoke_all_sessions(&self, user_id: i32) -> Result<()> {
        let sessions = self.get_user_sessions(user_id).await?;
        
        let mut conn = self.redis_pool.get().await?;
        for session in sessions {
            conn.del(format!("refresh_token:{}", session)).await?;
        }

        Ok(())
    }
}