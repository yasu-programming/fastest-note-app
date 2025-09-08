use crate::models::{Folder, CreateFolderRequest, UpdateFolderRequest, FolderResponse, FolderTreeResponse};
use crate::repositories::FolderRepository;
use anyhow::{Result, anyhow};
use redis::AsyncCommands;
use serde_json;

#[derive(Clone)]
pub struct FolderService {
    folder_repo: FolderRepository,
    redis_pool: deadpool_redis::Pool,
}

impl FolderService {
    pub fn new(folder_repo: FolderRepository, redis_pool: deadpool_redis::Pool) -> Self {
        Self {
            folder_repo,
            redis_pool,
        }
    }

    pub async fn create_folder(&self, request: CreateFolderRequest, user_id: i32) -> Result<FolderResponse> {
        // Validate folder name
        if request.name.trim().is_empty() {
            return Err(anyhow!("Folder name cannot be empty"));
        }

        if request.name.len() > 255 {
            return Err(anyhow!("Folder name cannot exceed 255 characters"));
        }

        // Check for invalid characters
        if request.name.contains('/') || request.name.contains('\\') {
            return Err(anyhow!("Folder name cannot contain / or \\ characters"));
        }

        let folder = self.folder_repo.create_folder(request, user_id).await?;

        // Invalidate cache
        self.invalidate_user_folder_cache(user_id).await?;

        Ok(folder.into())
    }

    pub async fn get_folder(&self, folder_id: i32, user_id: i32) -> Result<Option<FolderResponse>> {
        let folder = self.folder_repo.find_by_id(folder_id, user_id).await?;
        Ok(folder.map(|f| f.into()))
    }

    pub async fn get_user_folders(&self, user_id: i32) -> Result<Vec<FolderResponse>> {
        // Try to get from cache first
        let cache_key = format!("user_folders:{}", user_id);
        
        if let Ok(mut conn) = self.redis_pool.get().await {
            if let Ok(cached_data) = conn.get::<_, String>(&cache_key).await {
                if let Ok(folders) = serde_json::from_str::<Vec<FolderResponse>>(&cached_data) {
                    return Ok(folders);
                }
            }
        }

        // Get from database
        let folders = self.folder_repo.find_by_user(user_id).await?;
        let folder_responses: Vec<FolderResponse> = folders.into_iter().map(|f| f.into()).collect();

        // Cache the result
        if let Ok(mut conn) = self.redis_pool.get().await {
            let serialized = serde_json::to_string(&folder_responses).unwrap_or_default();
            let _: Result<(), _> = conn.setex(&cache_key, 300, serialized).await; // 5 minute cache
        }

        Ok(folder_responses)
    }

    pub async fn get_folder_tree(&self, user_id: i32) -> Result<Vec<FolderTreeResponse>> {
        // Try to get from cache first
        let cache_key = format!("folder_tree:{}", user_id);
        
        if let Ok(mut conn) = self.redis_pool.get().await {
            if let Ok(cached_data) = conn.get::<_, String>(&cache_key).await {
                if let Ok(tree) = serde_json::from_str::<Vec<FolderTreeResponse>>(&cached_data) {
                    return Ok(tree);
                }
            }
        }

        // Get from database
        let tree = self.folder_repo.get_folder_tree(user_id).await?;

        // Cache the result
        if let Ok(mut conn) = self.redis_pool.get().await {
            let serialized = serde_json::to_string(&tree).unwrap_or_default();
            let _: Result<(), _> = conn.setex(&cache_key, 300, serialized).await; // 5 minute cache
        }

        Ok(tree)
    }

    pub async fn get_folder_children(&self, parent_id: Option<i32>, user_id: i32) -> Result<Vec<FolderResponse>> {
        let folders = self.folder_repo.find_children(parent_id, user_id).await?;
        Ok(folders.into_iter().map(|f| f.into()).collect())
    }

    pub async fn update_folder(&self, folder_id: i32, request: UpdateFolderRequest, user_id: i32) -> Result<Option<FolderResponse>> {
        // Validate folder name
        if request.name.trim().is_empty() {
            return Err(anyhow!("Folder name cannot be empty"));
        }

        if request.name.len() > 255 {
            return Err(anyhow!("Folder name cannot exceed 255 characters"));
        }

        // Check for invalid characters
        if request.name.contains('/') || request.name.contains('\\') {
            return Err(anyhow!("Folder name cannot contain / or \\ characters"));
        }

        let updated_folder = self.folder_repo.update_folder(folder_id, request, user_id).await?;

        if updated_folder.is_some() {
            // Invalidate cache
            self.invalidate_user_folder_cache(user_id).await?;
        }

        Ok(updated_folder.map(|f| f.into()))
    }

    pub async fn delete_folder(&self, folder_id: i32, user_id: i32) -> Result<bool> {
        // Check if folder has any content that would be deleted
        let stats = self.folder_repo.get_folder_stats(folder_id, user_id).await?;
        
        if let Some((note_count, subfolder_count)) = stats {
            if note_count > 0 || subfolder_count > 0 {
                // In a production system, you might want to require explicit confirmation
                // or move contents to a "Trash" folder instead of permanent deletion
            }
        }

        let deleted = self.folder_repo.delete_folder(folder_id, user_id).await?;

        if deleted {
            // Invalidate cache
            self.invalidate_user_folder_cache(user_id).await?;
        }

        Ok(deleted)
    }

    pub async fn move_folder(&self, folder_id: i32, new_parent_id: Option<i32>, user_id: i32) -> Result<Option<FolderResponse>> {
        // Get current folder info
        let current_folder = self.folder_repo.find_by_id(folder_id, user_id).await?;
        let current_folder = match current_folder {
            Some(f) => f,
            None => return Ok(None),
        };

        // Create update request with current name but new parent
        let update_request = UpdateFolderRequest {
            name: current_folder.name,
            parent_id: new_parent_id,
        };

        self.update_folder(folder_id, update_request, user_id).await
    }

    pub async fn get_folder_stats(&self, folder_id: i32, user_id: i32) -> Result<Option<(i64, i64)>> {
        self.folder_repo.get_folder_stats(folder_id, user_id).await
    }

    pub async fn validate_folder_path(&self, folder_id: i32, user_id: i32) -> Result<bool> {
        // Check if folder exists and path is valid
        let folder = self.folder_repo.find_by_id(folder_id, user_id).await?;
        
        match folder {
            Some(folder) => {
                // Validate that depth matches path structure
                let expected_depth = if folder.path.is_empty() {
                    1
                } else {
                    folder.path.matches('/').count() as i32 + 1
                };
                
                Ok(folder.depth == expected_depth)
            },
            None => Ok(false),
        }
    }

    pub async fn search_folders(&self, query: &str, user_id: i32) -> Result<Vec<FolderResponse>> {
        if query.trim().is_empty() {
            return Ok(vec![]);
        }

        // Get all user folders and filter by name
        let all_folders = self.folder_repo.find_by_user(user_id).await?;
        let query_lower = query.to_lowercase();
        
        let matching_folders: Vec<FolderResponse> = all_folders
            .into_iter()
            .filter(|folder| {
                folder.name.to_lowercase().contains(&query_lower) ||
                folder.path.to_lowercase().contains(&query_lower)
            })
            .map(|f| f.into())
            .collect();

        Ok(matching_folders)
    }

    pub async fn get_folder_breadcrumbs(&self, folder_id: i32, user_id: i32) -> Result<Vec<FolderResponse>> {
        let folder = self.folder_repo.find_by_id(folder_id, user_id).await?;
        
        let folder = match folder {
            Some(f) => f,
            None => return Ok(vec![]),
        };

        if folder.path.is_empty() {
            return Ok(vec![folder.into()]);
        }

        // Get all folders and build breadcrumbs from path
        let all_folders = self.folder_repo.find_by_user(user_id).await?;
        let path_parts: Vec<&str> = folder.path.split('/').collect();
        
        let mut breadcrumbs = Vec::new();
        let mut current_path = String::new();
        
        for (index, part) in path_parts.iter().enumerate() {
            if index > 0 {
                current_path.push('/');
            }
            current_path.push_str(part);
            
            if let Some(breadcrumb_folder) = all_folders.iter().find(|f| f.path == current_path) {
                breadcrumbs.push(breadcrumb_folder.clone().into());
            }
        }

        Ok(breadcrumbs)
    }

    pub async fn check_folder_access(&self, folder_id: i32, user_id: i32) -> Result<bool> {
        let folder = self.folder_repo.find_by_id(folder_id, user_id).await?;
        Ok(folder.is_some())
    }

    async fn invalidate_user_folder_cache(&self, user_id: i32) -> Result<()> {
        if let Ok(mut conn) = self.redis_pool.get().await {
            let cache_keys = vec![
                format!("user_folders:{}", user_id),
                format!("folder_tree:{}", user_id),
            ];
            
            for key in cache_keys {
                let _: Result<(), _> = conn.del(&key).await;
            }
        }
        
        Ok(())
    }

    pub async fn duplicate_folder(&self, folder_id: i32, user_id: i32, new_name: Option<String>) -> Result<Option<FolderResponse>> {
        // Get source folder
        let source_folder = self.folder_repo.find_by_id(folder_id, user_id).await?;
        let source_folder = match source_folder {
            Some(f) => f,
            None => return Ok(None),
        };

        // Generate new name if not provided
        let duplicate_name = new_name.unwrap_or_else(|| format!("{} (Copy)", source_folder.name));

        // Create duplicate folder
        let create_request = CreateFolderRequest {
            name: duplicate_name,
            parent_id: source_folder.parent_id,
        };

        let duplicated_folder = self.create_folder(create_request, user_id).await?;

        // Note: This is a shallow copy. In a full implementation, you might want to
        // recursively copy all subfolders and notes as well.

        Ok(Some(duplicated_folder))
    }
}