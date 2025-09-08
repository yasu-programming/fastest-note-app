-- Migration 002: Additional Performance Indexes and Optimizations
-- Optimizations for high-performance note-taking operations

-- Partial indexes for common query patterns
CREATE INDEX CONCURRENTLY idx_notes_recent_by_user 
    ON notes(user_id, updated_at DESC) 
    WHERE updated_at > NOW() - INTERVAL '30 days';

-- Index for root-level notes (folder_id IS NULL)
CREATE INDEX CONCURRENTLY idx_notes_root_level 
    ON notes(user_id, updated_at DESC) 
    WHERE folder_id IS NULL;

-- Index for root-level folders (parent_folder_id IS NULL)  
CREATE INDEX CONCURRENTLY idx_folders_root_level
    ON folders(user_id, name)
    WHERE parent_folder_id IS NULL;

-- Composite index for folder hierarchy traversal
CREATE INDEX CONCURRENTLY idx_folders_hierarchy
    ON folders(user_id, parent_folder_id, level, name);

-- Index for search by title only (faster than full-text for exact matches)
CREATE INDEX CONCURRENTLY idx_notes_title_search
    ON notes USING GIN (to_tsvector('english', title));

-- Additional constraints for data integrity

-- Prevent circular references in folder hierarchy
CREATE OR REPLACE FUNCTION prevent_circular_folder_reference()
RETURNS TRIGGER AS $$
BEGIN
    -- Check if parent folder would create circular reference
    IF NEW.parent_folder_id IS NOT NULL THEN
        -- Check if any descendant folder is being set as parent
        IF EXISTS (
            WITH RECURSIVE folder_tree AS (
                -- Start from current folder's children
                SELECT id, parent_folder_id, 1 as depth
                FROM folders 
                WHERE parent_folder_id = NEW.id AND user_id = NEW.user_id
                
                UNION ALL
                
                -- Recursively find all descendants
                SELECT f.id, f.parent_folder_id, ft.depth + 1
                FROM folders f
                INNER JOIN folder_tree ft ON f.parent_folder_id = ft.id
                WHERE ft.depth < 20 -- Prevent infinite recursion
            )
            SELECT 1 FROM folder_tree WHERE id = NEW.parent_folder_id
        ) THEN
            RAISE EXCEPTION 'Cannot set parent folder: would create circular reference';
        END IF;
    END IF;
    
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER prevent_circular_folder_reference_trigger
    BEFORE UPDATE ON folders
    FOR EACH ROW 
    WHEN (OLD.parent_folder_id IS DISTINCT FROM NEW.parent_folder_id)
    EXECUTE FUNCTION prevent_circular_folder_reference();

-- Function for efficient folder tree queries
CREATE OR REPLACE FUNCTION get_folder_tree(p_user_id UUID, p_parent_id UUID DEFAULT NULL)
RETURNS TABLE (
    id UUID,
    name VARCHAR(255),
    path TEXT,
    level INTEGER,
    note_count BIGINT,
    has_children BOOLEAN
) AS $$
BEGIN
    RETURN QUERY
    WITH folder_notes AS (
        SELECT folder_id, COUNT(*) as note_count
        FROM notes 
        WHERE user_id = p_user_id
        GROUP BY folder_id
    )
    SELECT 
        f.id,
        f.name,
        f.path,
        f.level,
        COALESCE(fn.note_count, 0) as note_count,
        EXISTS(SELECT 1 FROM folders cf WHERE cf.parent_folder_id = f.id) as has_children
    FROM folders f
    LEFT JOIN folder_notes fn ON f.id = fn.folder_id
    WHERE f.user_id = p_user_id
    AND f.parent_folder_id IS NOT DISTINCT FROM p_parent_id
    ORDER BY f.name;
END;
$$ language 'plpgsql';

-- Function for full-text search across notes
CREATE OR REPLACE FUNCTION search_notes(
    p_user_id UUID,
    p_query TEXT,
    p_limit INTEGER DEFAULT 50,
    p_offset INTEGER DEFAULT 0
)
RETURNS TABLE (
    id UUID,
    title VARCHAR(500),
    content_preview TEXT,
    folder_id UUID,
    folder_name VARCHAR(255),
    updated_at TIMESTAMPTZ,
    rank REAL
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        n.id,
        n.title,
        LEFT(COALESCE(n.content, ''), 200) as content_preview,
        n.folder_id,
        f.name as folder_name,
        n.updated_at,
        ts_rank(to_tsvector('english', n.title || ' ' || COALESCE(n.content, '')), 
                plainto_tsquery('english', p_query)) as rank
    FROM notes n
    LEFT JOIN folders f ON n.folder_id = f.id
    WHERE n.user_id = p_user_id
    AND to_tsvector('english', n.title || ' ' || COALESCE(n.content, '')) 
        @@ plainto_tsquery('english', p_query)
    ORDER BY rank DESC, n.updated_at DESC
    LIMIT p_limit
    OFFSET p_offset;
END;
$$ language 'plpgsql';

-- Materialized view for user statistics (refresh periodically)
CREATE MATERIALIZED VIEW user_statistics AS
SELECT 
    u.id as user_id,
    u.email,
    u.created_at as user_created_at,
    COUNT(DISTINCT f.id) as total_folders,
    COUNT(DISTINCT n.id) as total_notes,
    SUM(n.content_size) as total_content_size,
    MAX(n.updated_at) as last_note_update,
    AVG(f.level) as avg_folder_depth
FROM users u
LEFT JOIN folders f ON u.id = f.user_id  
LEFT JOIN notes n ON u.id = n.user_id
GROUP BY u.id, u.email, u.created_at;

-- Index on the materialized view
CREATE INDEX idx_user_statistics_user_id ON user_statistics(user_id);

-- Function to refresh user statistics (call periodically)
CREATE OR REPLACE FUNCTION refresh_user_statistics()
RETURNS VOID AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY user_statistics;
END;
$$ language 'plpgsql';