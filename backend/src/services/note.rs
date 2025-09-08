use crate::models::{Note, CreateNoteRequest, UpdateNoteRequest, NoteResponse, NoteListResponse, SearchResult, SearchNotesQuery};
use crate::repositories::NoteRepository;
use crate::services::websocket::WebSocketService;
use anyhow::{Result, anyhow};
use redis::AsyncCommands;
use serde_json;
use std::sync::Arc;

#[derive(Clone)]
pub struct NoteService {
    note_repo: NoteRepository,
    redis_pool: deadpool_redis::Pool,
    websocket_service: Option<Arc<WebSocketService>>,
}

impl NoteService {
    pub fn new(note_repo: NoteRepository, redis_pool: deadpool_redis::Pool) -> Self {
        Self {
            note_repo,
            redis_pool,
            websocket_service: None,
        }
    }

    pub fn with_websocket_service(mut self, websocket_service: Arc<WebSocketService>) -> Self {
        self.websocket_service = Some(websocket_service);
        self
    }

    pub async fn create_note(&self, request: CreateNoteRequest, user_id: i32) -> Result<NoteResponse> {
        // Validate note content
        self.validate_note_content(&request.title, &request.content)?;

        let note = self.note_repo.create_note(request, user_id).await?;

        // Invalidate cache
        self.invalidate_user_note_cache(user_id).await?;

        // Notify via WebSocket
        if let Some(ws_service) = &self.websocket_service {
            let message = serde_json::json!({
                "type": "note_created",
                "data": NoteResponse::from(note.clone())
            });
            let _ = ws_service.broadcast_to_user(user_id, message).await;
        }

        Ok(note.into())
    }

    pub async fn get_note(&self, note_id: i32, user_id: i32) -> Result<Option<NoteResponse>> {
        // Try to get from cache first
        let cache_key = format!("note:{}:{}", user_id, note_id);
        
        if let Ok(mut conn) = self.redis_pool.get().await {
            if let Ok(cached_data) = conn.get::<_, String>(&cache_key).await {
                if let Ok(note) = serde_json::from_str::<NoteResponse>(&cached_data) {
                    return Ok(Some(note));
                }
            }
        }

        // Get from database
        let note = self.note_repo.find_by_id(note_id, user_id).await?;
        
        if let Some(note) = note {
            let note_response = NoteResponse::from(note);
            
            // Cache the result
            if let Ok(mut conn) = self.redis_pool.get().await {
                let serialized = serde_json::to_string(&note_response).unwrap_or_default();
                let _: Result<(), _> = conn.setex(&cache_key, 600, serialized).await; // 10 minute cache
            }
            
            Ok(Some(note_response))
        } else {
            Ok(None)
        }
    }

    pub async fn get_user_notes(&self, user_id: i32, limit: Option<i32>, offset: Option<i32>) -> Result<Vec<NoteListResponse>> {
        let notes = self.note_repo.find_by_user(user_id, limit, offset).await?;
        Ok(notes.into_iter().map(|n| n.into()).collect())
    }

    pub async fn get_folder_notes(&self, folder_id: i32, user_id: i32, limit: Option<i32>, offset: Option<i32>) -> Result<Vec<NoteListResponse>> {
        let notes = self.note_repo.find_by_folder(folder_id, user_id, limit, offset).await?;
        Ok(notes.into_iter().map(|n| n.into()).collect())
    }

    pub async fn update_note(&self, note_id: i32, request: UpdateNoteRequest, user_id: i32) -> Result<Option<NoteResponse>> {
        // Validate note content
        self.validate_note_content(&request.title, &request.content)?;

        let updated_note = self.note_repo.update_note(note_id, request, user_id).await?;

        if let Some(note) = &updated_note {
            // Invalidate cache
            self.invalidate_note_cache(user_id, note_id).await?;

            // Notify via WebSocket
            if let Some(ws_service) = &self.websocket_service {
                let message = serde_json::json!({
                    "type": "note_updated",
                    "data": NoteResponse::from(note.clone())
                });
                let _ = ws_service.broadcast_to_user(user_id, message).await;
            }
        }

        Ok(updated_note.map(|n| n.into()))
    }

    pub async fn delete_note(&self, note_id: i32, user_id: i32) -> Result<bool> {
        let deleted = self.note_repo.delete_note(note_id, user_id).await?;

        if deleted {
            // Invalidate cache
            self.invalidate_note_cache(user_id, note_id).await?;

            // Notify via WebSocket
            if let Some(ws_service) = &self.websocket_service {
                let message = serde_json::json!({
                    "type": "note_deleted",
                    "data": {
                        "id": note_id
                    }
                });
                let _ = ws_service.broadcast_to_user(user_id, message).await;
            }
        }

        Ok(deleted)
    }

    pub async fn search_notes(&self, query: SearchNotesQuery, user_id: i32) -> Result<Vec<SearchResult>> {
        // Validate search query
        if query.q.trim().is_empty() {
            return Ok(vec![]);
        }

        if query.q.len() > 500 {
            return Err(anyhow!("Search query too long"));
        }

        // Try to get from cache first (for exact same query)
        let cache_key = format!("search:{}:{}", user_id, self.generate_query_hash(&query));
        
        if let Ok(mut conn) = self.redis_pool.get().await {
            if let Ok(cached_data) = conn.get::<_, String>(&cache_key).await {
                if let Ok(results) = serde_json::from_str::<Vec<SearchResult>>(&cached_data) {
                    return Ok(results);
                }
            }
        }

        // Search in database
        let results = self.note_repo.search_notes(query, user_id).await?;

        // Cache the results
        if let Ok(mut conn) = self.redis_pool.get().await {
            let serialized = serde_json::to_string(&results).unwrap_or_default();
            let _: Result<(), _> = conn.setex(&cache_key, 300, serialized).await; // 5 minute cache
        }

        Ok(results)
    }

    pub async fn get_recent_notes(&self, user_id: i32, limit: Option<i32>) -> Result<Vec<NoteListResponse>> {
        let notes = self.note_repo.get_recent_notes(user_id, limit).await?;
        Ok(notes.into_iter().map(|n| n.into()).collect())
    }

    pub async fn move_notes_to_folder(&self, note_ids: Vec<i32>, folder_id: Option<i32>, user_id: i32) -> Result<usize> {
        if note_ids.is_empty() {
            return Ok(0);
        }

        if note_ids.len() > 100 {
            return Err(anyhow!("Cannot move more than 100 notes at once"));
        }

        let moved_count = self.note_repo.move_notes_to_folder(note_ids.clone(), folder_id, user_id).await?;

        if moved_count > 0 {
            // Invalidate cache for affected notes
            for note_id in note_ids {
                self.invalidate_note_cache(user_id, note_id).await?;
            }

            // Notify via WebSocket
            if let Some(ws_service) = &self.websocket_service {
                let message = serde_json::json!({
                    "type": "notes_moved",
                    "data": {
                        "note_ids": note_ids,
                        "folder_id": folder_id,
                        "count": moved_count
                    }
                });
                let _ = ws_service.broadcast_to_user(user_id, message).await;
            }
        }

        Ok(moved_count)
    }

    pub async fn duplicate_note(&self, note_id: i32, user_id: i32) -> Result<Option<NoteResponse>> {
        let duplicated_note = self.note_repo.duplicate_note(note_id, user_id).await?;

        if let Some(note) = &duplicated_note {
            // Invalidate cache
            self.invalidate_user_note_cache(user_id).await?;

            // Notify via WebSocket
            if let Some(ws_service) = &self.websocket_service {
                let message = serde_json::json!({
                    "type": "note_created",
                    "data": NoteResponse::from(note.clone())
                });
                let _ = ws_service.broadcast_to_user(user_id, message).await;
            }
        }

        Ok(duplicated_note.map(|n| n.into()))
    }

    pub async fn get_note_count(&self, user_id: i32, folder_id: Option<i32>) -> Result<i64> {
        self.note_repo.get_note_count(user_id, folder_id).await
    }

    pub async fn batch_delete_notes(&self, note_ids: Vec<i32>, user_id: i32) -> Result<usize> {
        if note_ids.is_empty() {
            return Ok(0);
        }

        if note_ids.len() > 100 {
            return Err(anyhow!("Cannot delete more than 100 notes at once"));
        }

        let mut deleted_count = 0;
        for note_id in &note_ids {
            if self.note_repo.delete_note(*note_id, user_id).await? {
                deleted_count += 1;
                self.invalidate_note_cache(user_id, *note_id).await?;
            }
        }

        if deleted_count > 0 {
            // Notify via WebSocket
            if let Some(ws_service) = &self.websocket_service {
                let message = serde_json::json!({
                    "type": "notes_deleted",
                    "data": {
                        "note_ids": note_ids,
                        "count": deleted_count
                    }
                });
                let _ = ws_service.broadcast_to_user(user_id, message).await;
            }
        }

        Ok(deleted_count)
    }

    pub async fn get_note_statistics(&self, user_id: i32) -> Result<serde_json::Value> {
        let total_notes = self.note_repo.get_note_count(user_id, None).await?;
        
        // Get notes without folder (root level)
        let root_notes = self.note_repo.get_note_count(user_id, Some(0)).await.unwrap_or(0);
        
        // Get recent notes for activity
        let recent_notes = self.note_repo.get_recent_notes(user_id, Some(5)).await?;

        Ok(serde_json::json!({
            "total_notes": total_notes,
            "root_notes": root_notes,
            "recent_activity": recent_notes.len(),
            "last_activity": recent_notes.first().map(|n| n.updated_at)
        }))
    }

    fn validate_note_content(&self, title: &str, content: &str) -> Result<()> {
        if title.trim().is_empty() {
            return Err(anyhow!("Note title cannot be empty"));
        }

        if title.len() > 255 {
            return Err(anyhow!("Note title cannot exceed 255 characters"));
        }

        if content.len() > 1_048_576 {
            return Err(anyhow!("Note content exceeds 1MB limit"));
        }

        Ok(())
    }

    fn generate_query_hash(&self, query: &SearchNotesQuery) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        query.q.hash(&mut hasher);
        query.folder_id.hash(&mut hasher);
        query.limit.hash(&mut hasher);
        query.offset.hash(&mut hasher);
        
        format!("{:x}", hasher.finish())
    }

    async fn invalidate_note_cache(&self, user_id: i32, note_id: i32) -> Result<()> {
        if let Ok(mut conn) = self.redis_pool.get().await {
            let cache_keys = vec![
                format!("note:{}:{}", user_id, note_id),
            ];
            
            for key in cache_keys {
                let _: Result<(), _> = conn.del(&key).await;
            }
        }
        
        // Also invalidate user-wide caches
        self.invalidate_user_note_cache(user_id).await
    }

    async fn invalidate_user_note_cache(&self, user_id: i32) -> Result<()> {
        if let Ok(mut conn) = self.redis_pool.get().await {
            // Invalidate search cache for this user
            let search_pattern = format!("search:{}:*", user_id);
            if let Ok(search_keys) = conn.keys::<_, Vec<String>>(search_pattern).await {
                for key in search_keys {
                    let _: Result<(), _> = conn.del(&key).await;
                }
            }
        }
        
        Ok(())
    }

    pub async fn get_note_analytics(&self, user_id: i32, days: i32) -> Result<serde_json::Value> {
        // This would typically involve more complex queries
        // For now, return basic analytics
        
        let total_notes = self.note_repo.get_note_count(user_id, None).await?;
        let recent_notes = self.note_repo.get_recent_notes(user_id, Some(days)).await?;
        
        Ok(serde_json::json!({
            "total_notes": total_notes,
            "notes_in_period": recent_notes.len(),
            "period_days": days,
            "average_per_day": recent_notes.len() as f64 / days as f64
        }))
    }
}