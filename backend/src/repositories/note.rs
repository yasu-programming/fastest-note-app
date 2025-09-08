use crate::models::{Note, CreateNoteRequest, UpdateNoteRequest, SearchResult, SearchNotesQuery};
use anyhow::{Result, anyhow};
use sqlx::PgPool;

#[derive(Clone)]
pub struct NoteRepository {
    pool: PgPool,
}

impl NoteRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_note(&self, request: CreateNoteRequest, user_id: i32) -> Result<Note> {
        let mut tx = self.pool.begin().await?;

        // Validate content size (1MB limit)
        if request.content.len() > 1_048_576 {
            return Err(anyhow!("Note content exceeds 1MB limit"));
        }

        // Validate title length
        if request.title.is_empty() || request.title.len() > 255 {
            return Err(anyhow!("Title must be between 1 and 255 characters"));
        }

        // If folder_id is provided, verify it exists and belongs to user
        if let Some(folder_id) = request.folder_id {
            let folder_exists = sqlx::query!(
                r#"
                SELECT EXISTS(SELECT 1 FROM folders WHERE id = $1 AND user_id = $2)
                "#,
                folder_id,
                user_id
            )
            .fetch_one(&mut *tx)
            .await?;

            if !folder_exists.exists.unwrap_or(false) {
                return Err(anyhow!("Folder not found or access denied"));
            }
        }

        let note = sqlx::query_as!(
            Note,
            r#"
            INSERT INTO notes (title, content, folder_id, user_id, version)
            VALUES ($1, $2, $3, $4, 1)
            RETURNING id, title, content, folder_id, user_id, version, created_at, updated_at
            "#,
            request.title,
            request.content,
            request.folder_id,
            user_id
        )
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(note)
    }

    pub async fn find_by_id(&self, note_id: i32, user_id: i32) -> Result<Option<Note>> {
        let note = sqlx::query_as!(
            Note,
            r#"
            SELECT id, title, content, folder_id, user_id, version, created_at, updated_at
            FROM notes
            WHERE id = $1 AND user_id = $2
            "#,
            note_id,
            user_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(note)
    }

    pub async fn find_by_user(&self, user_id: i32, limit: Option<i32>, offset: Option<i32>) -> Result<Vec<Note>> {
        let limit = limit.unwrap_or(50).min(100); // Cap at 100
        let offset = offset.unwrap_or(0).max(0);

        let notes = sqlx::query_as!(
            Note,
            r#"
            SELECT id, title, content, folder_id, user_id, version, created_at, updated_at
            FROM notes
            WHERE user_id = $1
            ORDER BY updated_at DESC
            LIMIT $2 OFFSET $3
            "#,
            user_id,
            limit as i64,
            offset as i64
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(notes)
    }

    pub async fn find_by_folder(&self, folder_id: i32, user_id: i32, limit: Option<i32>, offset: Option<i32>) -> Result<Vec<Note>> {
        let limit = limit.unwrap_or(50).min(100);
        let offset = offset.unwrap_or(0).max(0);

        let notes = sqlx::query_as!(
            Note,
            r#"
            SELECT id, title, content, folder_id, user_id, version, created_at, updated_at
            FROM notes
            WHERE folder_id = $1 AND user_id = $2
            ORDER BY updated_at DESC
            LIMIT $3 OFFSET $4
            "#,
            folder_id,
            user_id,
            limit as i64,
            offset as i64
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(notes)
    }

    pub async fn update_note(&self, note_id: i32, request: UpdateNoteRequest, user_id: i32) -> Result<Option<Note>> {
        let mut tx = self.pool.begin().await?;

        // Validate content size (1MB limit)
        if request.content.len() > 1_048_576 {
            return Err(anyhow!("Note content exceeds 1MB limit"));
        }

        // Validate title length
        if request.title.is_empty() || request.title.len() > 255 {
            return Err(anyhow!("Title must be between 1 and 255 characters"));
        }

        // Check current version for optimistic locking
        let current_note = sqlx::query!(
            r#"
            SELECT version
            FROM notes
            WHERE id = $1 AND user_id = $2
            "#,
            note_id,
            user_id
        )
        .fetch_optional(&mut *tx)
        .await?;

        let current_note = match current_note {
            Some(n) => n,
            None => return Ok(None),
        };

        if current_note.version != request.version {
            return Err(anyhow!("Optimistic locking failed: note was modified by another process"));
        }

        // If folder_id is provided, verify it exists and belongs to user
        if let Some(folder_id) = request.folder_id {
            let folder_exists = sqlx::query!(
                r#"
                SELECT EXISTS(SELECT 1 FROM folders WHERE id = $1 AND user_id = $2)
                "#,
                folder_id,
                user_id
            )
            .fetch_one(&mut *tx)
            .await?;

            if !folder_exists.exists.unwrap_or(false) {
                return Err(anyhow!("Folder not found or access denied"));
            }
        }

        let updated_note = sqlx::query_as!(
            Note,
            r#"
            UPDATE notes
            SET title = $1, content = $2, folder_id = $3, version = version + 1, updated_at = NOW()
            WHERE id = $4 AND user_id = $5
            RETURNING id, title, content, folder_id, user_id, version, created_at, updated_at
            "#,
            request.title,
            request.content,
            request.folder_id,
            note_id,
            user_id
        )
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(Some(updated_note))
    }

    pub async fn delete_note(&self, note_id: i32, user_id: i32) -> Result<bool> {
        let result = sqlx::query!(
            r#"
            DELETE FROM notes
            WHERE id = $1 AND user_id = $2
            "#,
            note_id,
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn search_notes(&self, query: SearchNotesQuery, user_id: i32) -> Result<Vec<SearchResult>> {
        let limit = query.limit.unwrap_or(20).min(100);
        let offset = query.offset.unwrap_or(0).max(0);

        let search_results = if let Some(folder_id) = query.folder_id {
            // Search within specific folder
            sqlx::query!(
                r#"
                SELECT 
                    n.id,
                    n.title,
                    ts_headline('english', n.content, websearch_to_tsquery('english', $1)) as content_snippet,
                    n.folder_id,
                    f.name as folder_name,
                    n.created_at,
                    n.updated_at,
                    ts_rank(n.search_vector, websearch_to_tsquery('english', $1)) as rank
                FROM notes n
                LEFT JOIN folders f ON n.folder_id = f.id
                WHERE n.user_id = $2 
                    AND n.folder_id = $3
                    AND n.search_vector @@ websearch_to_tsquery('english', $1)
                ORDER BY ts_rank(n.search_vector, websearch_to_tsquery('english', $1)) DESC
                LIMIT $4 OFFSET $5
                "#,
                query.q,
                user_id,
                folder_id,
                limit as i64,
                offset as i64
            )
            .fetch_all(&self.pool)
            .await?
        } else {
            // Search all notes
            sqlx::query!(
                r#"
                SELECT 
                    n.id,
                    n.title,
                    ts_headline('english', n.content, websearch_to_tsquery('english', $1)) as content_snippet,
                    n.folder_id,
                    f.name as folder_name,
                    n.created_at,
                    n.updated_at,
                    ts_rank(n.search_vector, websearch_to_tsquery('english', $1)) as rank
                FROM notes n
                LEFT JOIN folders f ON n.folder_id = f.id
                WHERE n.user_id = $2 
                    AND n.search_vector @@ websearch_to_tsquery('english', $1)
                ORDER BY ts_rank(n.search_vector, websearch_to_tsquery('english', $1)) DESC
                LIMIT $3 OFFSET $4
                "#,
                query.q,
                user_id,
                limit as i64,
                offset as i64
            )
            .fetch_all(&self.pool)
            .await?
        };

        let results = search_results
            .into_iter()
            .map(|row| SearchResult {
                id: row.id,
                title: row.title,
                content_snippet: row.content_snippet.unwrap_or_default(),
                folder_id: row.folder_id,
                folder_name: row.folder_name,
                created_at: row.created_at,
                updated_at: row.updated_at,
                rank: row.rank,
            })
            .collect();

        Ok(results)
    }

    pub async fn get_recent_notes(&self, user_id: i32, limit: Option<i32>) -> Result<Vec<Note>> {
        let limit = limit.unwrap_or(10).min(50);

        let notes = sqlx::query_as!(
            Note,
            r#"
            SELECT id, title, content, folder_id, user_id, version, created_at, updated_at
            FROM notes
            WHERE user_id = $1
            ORDER BY updated_at DESC
            LIMIT $2
            "#,
            user_id,
            limit as i64
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(notes)
    }

    pub async fn move_notes_to_folder(&self, note_ids: Vec<i32>, folder_id: Option<i32>, user_id: i32) -> Result<usize> {
        let mut tx = self.pool.begin().await?;

        // If folder_id is provided, verify it exists and belongs to user
        if let Some(folder_id) = folder_id {
            let folder_exists = sqlx::query!(
                r#"
                SELECT EXISTS(SELECT 1 FROM folders WHERE id = $1 AND user_id = $2)
                "#,
                folder_id,
                user_id
            )
            .fetch_one(&mut *tx)
            .await?;

            if !folder_exists.exists.unwrap_or(false) {
                return Err(anyhow!("Folder not found or access denied"));
            }
        }

        let result = sqlx::query!(
            r#"
            UPDATE notes
            SET folder_id = $1, updated_at = NOW()
            WHERE id = ANY($2) AND user_id = $3
            "#,
            folder_id,
            &note_ids,
            user_id
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(result.rows_affected() as usize)
    }

    pub async fn get_note_count(&self, user_id: i32, folder_id: Option<i32>) -> Result<i64> {
        let count = if let Some(folder_id) = folder_id {
            sqlx::query!(
                r#"
                SELECT COUNT(*) as count
                FROM notes
                WHERE user_id = $1 AND folder_id = $2
                "#,
                user_id,
                folder_id
            )
            .fetch_one(&self.pool)
            .await?
        } else {
            sqlx::query!(
                r#"
                SELECT COUNT(*) as count
                FROM notes
                WHERE user_id = $1
                "#,
                user_id
            )
            .fetch_one(&self.pool)
            .await?
        };

        Ok(count.count.unwrap_or(0))
    }

    pub async fn duplicate_note(&self, note_id: i32, user_id: i32) -> Result<Option<Note>> {
        let mut tx = self.pool.begin().await?;

        // Get the original note
        let original_note = sqlx::query_as!(
            Note,
            r#"
            SELECT id, title, content, folder_id, user_id, version, created_at, updated_at
            FROM notes
            WHERE id = $1 AND user_id = $2
            "#,
            note_id,
            user_id
        )
        .fetch_optional(&mut *tx)
        .await?;

        let original_note = match original_note {
            Some(n) => n,
            None => return Ok(None),
        };

        // Create duplicate with modified title
        let duplicate_title = format!("{} (Copy)", original_note.title);
        
        let duplicate_note = sqlx::query_as!(
            Note,
            r#"
            INSERT INTO notes (title, content, folder_id, user_id, version)
            VALUES ($1, $2, $3, $4, 1)
            RETURNING id, title, content, folder_id, user_id, version, created_at, updated_at
            "#,
            duplicate_title,
            original_note.content,
            original_note.folder_id,
            user_id
        )
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(Some(duplicate_note))
    }
}