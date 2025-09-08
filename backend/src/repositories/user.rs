use crate::models::{User, CreateUserRequest};
use anyhow::Result;
use bcrypt::{hash, verify, DEFAULT_COST};
use sqlx::PgPool;

#[derive(Clone)]
pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_user(&self, request: CreateUserRequest) -> Result<User> {
        let password_hash = hash(&request.password, DEFAULT_COST)?;
        
        let user = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (email, password_hash)
            VALUES ($1, $2)
            RETURNING id, email, password_hash, created_at, updated_at
            "#,
            request.email,
            password_hash
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(user)
    }

    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>> {
        let user = sqlx::query_as!(
            User,
            r#"
            SELECT id, email, password_hash, created_at, updated_at
            FROM users
            WHERE email = $1
            "#,
            email
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    pub async fn find_by_id(&self, id: i32) -> Result<Option<User>> {
        let user = sqlx::query_as!(
            User,
            r#"
            SELECT id, email, password_hash, created_at, updated_at
            FROM users
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    pub async fn verify_password(&self, user: &User, password: &str) -> Result<bool> {
        let is_valid = verify(password, &user.password_hash)?;
        Ok(is_valid)
    }

    pub async fn update_password(&self, user_id: i32, new_password: &str) -> Result<()> {
        let password_hash = hash(new_password, DEFAULT_COST)?;
        
        sqlx::query!(
            r#"
            UPDATE users
            SET password_hash = $1, updated_at = NOW()
            WHERE id = $2
            "#,
            password_hash,
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_user(&self, user_id: i32) -> Result<bool> {
        let result = sqlx::query!(
            r#"
            DELETE FROM users
            WHERE id = $1
            "#,
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn email_exists(&self, email: &str) -> Result<bool> {
        let exists = sqlx::query!(
            r#"
            SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)
            "#,
            email
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(exists.exists.unwrap_or(false))
    }
}