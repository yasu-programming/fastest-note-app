use crate::models::{Folder, CreateFolderRequest, UpdateFolderRequest, FolderTreeResponse};
use anyhow::{Result, anyhow};
use sqlx::PgPool;
use std::collections::HashMap;

#[derive(Clone)]
pub struct FolderRepository {
    pool: PgPool,
}

impl FolderRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_folder(&self, request: CreateFolderRequest, user_id: i32) -> Result<Folder> {
        let mut tx = self.pool.begin().await?;

        // Get parent folder info if parent_id is provided
        let (parent_path, parent_depth) = if let Some(parent_id) = request.parent_id {
            let parent = sqlx::query!(
                r#"
                SELECT path, depth
                FROM folders
                WHERE id = $1 AND user_id = $2
                "#,
                parent_id,
                user_id
            )
            .fetch_optional(&mut *tx)
            .await?;

            match parent {
                Some(p) => (p.path, p.depth),
                None => return Err(anyhow!("Parent folder not found or access denied")),
            }
        } else {
            (String::new(), 0)
        };

        // Check depth limit
        let new_depth = parent_depth + 1;
        if new_depth > 10 {
            return Err(anyhow!("Maximum folder depth (10) exceeded"));
        }

        // Generate path
        let new_path = if parent_path.is_empty() {
            request.name.clone()
        } else {
            format!("{}/{}", parent_path, request.name)
        };

        // Check for duplicate names at the same level
        let duplicate_count = sqlx::query!(
            r#"
            SELECT COUNT(*) as count
            FROM folders
            WHERE parent_id = $1 AND name = $2 AND user_id = $3
            "#,
            request.parent_id,
            request.name,
            user_id
        )
        .fetch_one(&mut *tx)
        .await?;

        if duplicate_count.count.unwrap_or(0) > 0 {
            return Err(anyhow!("Folder with this name already exists at this level"));
        }

        let folder = sqlx::query_as!(
            Folder,
            r#"
            INSERT INTO folders (name, parent_id, path, depth, user_id)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, name, parent_id, path, depth, user_id, created_at, updated_at
            "#,
            request.name,
            request.parent_id,
            new_path,
            new_depth,
            user_id
        )
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(folder)
    }

    pub async fn find_by_id(&self, folder_id: i32, user_id: i32) -> Result<Option<Folder>> {
        let folder = sqlx::query_as!(
            Folder,
            r#"
            SELECT id, name, parent_id, path, depth, user_id, created_at, updated_at
            FROM folders
            WHERE id = $1 AND user_id = $2
            "#,
            folder_id,
            user_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(folder)
    }

    pub async fn find_by_user(&self, user_id: i32) -> Result<Vec<Folder>> {
        let folders = sqlx::query_as!(
            Folder,
            r#"
            SELECT id, name, parent_id, path, depth, user_id, created_at, updated_at
            FROM folders
            WHERE user_id = $1
            ORDER BY path
            "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(folders)
    }

    pub async fn find_children(&self, parent_id: Option<i32>, user_id: i32) -> Result<Vec<Folder>> {
        let folders = sqlx::query_as!(
            Folder,
            r#"
            SELECT id, name, parent_id, path, depth, user_id, created_at, updated_at
            FROM folders
            WHERE parent_id = $1 AND user_id = $2
            ORDER BY name
            "#,
            parent_id,
            user_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(folders)
    }

    pub async fn get_folder_tree(&self, user_id: i32) -> Result<Vec<FolderTreeResponse>> {
        // Get all folders for the user
        let folders = sqlx::query!(
            r#"
            SELECT f.id, f.name, f.parent_id, f.path, f.depth, f.created_at, f.updated_at,
                   COALESCE(COUNT(n.id), 0) as note_count
            FROM folders f
            LEFT JOIN notes n ON f.id = n.folder_id
            WHERE f.user_id = $1
            GROUP BY f.id, f.name, f.parent_id, f.path, f.depth, f.created_at, f.updated_at
            ORDER BY f.path
            "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await?;

        let mut folder_map: HashMap<i32, FolderTreeResponse> = HashMap::new();
        let mut root_folders = Vec::new();

        // First pass: create all folder responses
        for folder in folders {
            let folder_response = FolderTreeResponse {
                id: folder.id,
                name: folder.name,
                path: folder.path,
                depth: folder.depth,
                created_at: folder.created_at,
                updated_at: folder.updated_at,
                children: Vec::new(),
                note_count: folder.note_count.unwrap_or(0),
            };

            if folder.parent_id.is_none() {
                root_folders.push(folder_response.clone());
            }

            folder_map.insert(folder.id, folder_response);
        }

        // Second pass: build the tree structure
        let mut folders_to_process: Vec<_> = folder_map.values().cloned().collect();
        folders_to_process.sort_by_key(|f| f.depth);

        for folder in folders_to_process {
            if let Some(parent_folder) = folder_map.get_mut(&folder.id) {
                // This folder exists, add children if any
                let children: Vec<_> = folder_map
                    .values()
                    .filter(|f| {
                        // Find folders that should be children of this folder
                        f.path.starts_with(&format!("{}/", folder.path)) && 
                        f.depth == folder.depth + 1
                    })
                    .cloned()
                    .collect();

                parent_folder.children = children;
            }
        }

        Ok(root_folders)
    }

    pub async fn update_folder(&self, folder_id: i32, request: UpdateFolderRequest, user_id: i32) -> Result<Option<Folder>> {
        let mut tx = self.pool.begin().await?;

        // Check if folder exists and belongs to user
        let existing_folder = sqlx::query!(
            r#"
            SELECT id, parent_id, path, depth, name
            FROM folders
            WHERE id = $1 AND user_id = $2
            "#,
            folder_id,
            user_id
        )
        .fetch_optional(&mut *tx)
        .await?;

        let existing_folder = match existing_folder {
            Some(f) => f,
            None => return Ok(None),
        };

        // Prevent moving folder to be a child of itself or its descendants
        if let Some(new_parent_id) = request.parent_id {
            if new_parent_id == folder_id {
                return Err(anyhow!("Folder cannot be moved to itself"));
            }

            // Check if new parent is a descendant
            let descendant_check = sqlx::query!(
                r#"
                SELECT COUNT(*) as count
                FROM folders
                WHERE path LIKE $1 AND user_id = $2 AND id = $3
                "#,
                format!("{}%", existing_folder.path),
                user_id,
                new_parent_id
            )
            .fetch_one(&mut *tx)
            .await?;

            if descendant_check.count.unwrap_or(0) > 0 {
                return Err(anyhow!("Folder cannot be moved to its own descendant"));
            }
        }

        // Get new parent info
        let (parent_path, parent_depth) = if let Some(parent_id) = request.parent_id {
            let parent = sqlx::query!(
                r#"
                SELECT path, depth
                FROM folders
                WHERE id = $1 AND user_id = $2
                "#,
                parent_id,
                user_id
            )
            .fetch_optional(&mut *tx)
            .await?;

            match parent {
                Some(p) => (p.path, p.depth),
                None => return Err(anyhow!("Parent folder not found or access denied")),
            }
        } else {
            (String::new(), 0)
        };

        let new_depth = parent_depth + 1;
        if new_depth > 10 {
            return Err(anyhow!("Maximum folder depth (10) exceeded"));
        }

        // Generate new path
        let new_path = if parent_path.is_empty() {
            request.name.clone()
        } else {
            format!("{}/{}", parent_path, request.name)
        };

        // Check for duplicate names at the new level (excluding the current folder)
        let duplicate_count = sqlx::query!(
            r#"
            SELECT COUNT(*) as count
            FROM folders
            WHERE parent_id = $1 AND name = $2 AND user_id = $3 AND id != $4
            "#,
            request.parent_id,
            request.name,
            user_id,
            folder_id
        )
        .fetch_one(&mut *tx)
        .await?;

        if duplicate_count.count.unwrap_or(0) > 0 {
            return Err(anyhow!("Folder with this name already exists at this level"));
        }

        // Update the folder
        let updated_folder = sqlx::query_as!(
            Folder,
            r#"
            UPDATE folders
            SET name = $1, parent_id = $2, path = $3, depth = $4, updated_at = NOW()
            WHERE id = $5 AND user_id = $6
            RETURNING id, name, parent_id, path, depth, user_id, created_at, updated_at
            "#,
            request.name,
            request.parent_id,
            new_path,
            new_depth,
            folder_id,
            user_id
        )
        .fetch_one(&mut *tx)
        .await?;

        // Update paths of all descendant folders if path changed
        if new_path != existing_folder.path {
            sqlx::query!(
                r#"
                UPDATE folders
                SET path = REPLACE(path, $1, $2),
                    depth = depth + $3,
                    updated_at = NOW()
                WHERE path LIKE $4 AND user_id = $5 AND id != $6
                "#,
                existing_folder.path,
                new_path,
                new_depth - existing_folder.depth,
                format!("{}/%", existing_folder.path),
                user_id,
                folder_id
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(Some(updated_folder))
    }

    pub async fn delete_folder(&self, folder_id: i32, user_id: i32) -> Result<bool> {
        let mut tx = self.pool.begin().await?;

        // Check if folder exists and belongs to user
        let folder = sqlx::query!(
            r#"
            SELECT path
            FROM folders
            WHERE id = $1 AND user_id = $2
            "#,
            folder_id,
            user_id
        )
        .fetch_optional(&mut *tx)
        .await?;

        let folder = match folder {
            Some(f) => f,
            None => return Ok(false),
        };

        // Delete all descendant folders (cascade delete)
        sqlx::query!(
            r#"
            DELETE FROM folders
            WHERE (path LIKE $1 OR id = $2) AND user_id = $3
            "#,
            format!("{}/%", folder.path),
            folder_id,
            user_id
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(true)
    }

    pub async fn get_folder_stats(&self, folder_id: i32, user_id: i32) -> Result<Option<(i64, i64)>> {
        let stats = sqlx::query!(
            r#"
            SELECT 
                COUNT(DISTINCT n.id) as note_count,
                COUNT(DISTINCT sf.id) as subfolder_count
            FROM folders f
            LEFT JOIN notes n ON f.id = n.folder_id
            LEFT JOIN folders sf ON f.id = sf.parent_id
            WHERE f.id = $1 AND f.user_id = $2
            GROUP BY f.id
            "#,
            folder_id,
            user_id
        )
        .fetch_optional(&self.pool)
        .await?;

        match stats {
            Some(s) => Ok(Some((
                s.note_count.unwrap_or(0),
                s.subfolder_count.unwrap_or(0),
            ))),
            None => Ok(None),
        }
    }
}