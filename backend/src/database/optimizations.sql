-- Database Performance Optimizations for Fastest Note App
-- Target: Sub-200ms query performance for all operations

-- =============================================
-- INDEXES FOR PERFORMANCE
-- =============================================

-- Users table indexes
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_users_created_at ON users(created_at);

-- Notes table indexes (most critical for performance)
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_notes_user_id ON notes(user_id);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_notes_folder_id ON notes(folder_id);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_notes_updated_at ON notes(updated_at DESC);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_notes_created_at ON notes(created_at DESC);

-- Composite indexes for common query patterns
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_notes_user_folder ON notes(user_id, folder_id);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_notes_user_updated ON notes(user_id, updated_at DESC);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_notes_folder_updated ON notes(folder_id, updated_at DESC);

-- Full-text search index for notes content
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_notes_search ON notes 
USING gin(to_tsvector('english', title || ' ' || content));

-- Folders table indexes
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_folders_user_id ON folders(user_id);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_folders_parent_id ON folders(parent_id);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_folders_path ON folders(path);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_folders_user_parent ON folders(user_id, parent_id);

-- Partial indexes for specific conditions
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_notes_active ON notes(user_id, updated_at DESC) 
WHERE deleted_at IS NULL;

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_folders_active ON folders(user_id, parent_id) 
WHERE deleted_at IS NULL;

-- =============================================
-- MATERIALIZED VIEWS FOR PERFORMANCE
-- =============================================

-- User statistics materialized view
CREATE MATERIALIZED VIEW IF NOT EXISTS user_stats AS
SELECT 
    u.id as user_id,
    u.email,
    COUNT(n.id) as note_count,
    COUNT(f.id) as folder_count,
    MAX(n.updated_at) as last_note_update,
    MAX(f.updated_at) as last_folder_update,
    SUM(LENGTH(n.content)) as total_content_size
FROM users u
LEFT JOIN notes n ON u.id = n.user_id AND n.deleted_at IS NULL
LEFT JOIN folders f ON u.id = f.user_id AND f.deleted_at IS NULL
GROUP BY u.id, u.email;

-- Create unique index on materialized view
CREATE UNIQUE INDEX IF NOT EXISTS idx_user_stats_user_id ON user_stats(user_id);

-- Folder hierarchy materialized view for fast tree operations
CREATE MATERIALIZED VIEW IF NOT EXISTS folder_hierarchy AS
WITH RECURSIVE folder_tree AS (
    -- Base case: root folders
    SELECT 
        id,
        name,
        parent_id,
        user_id,
        path,
        0 as depth,
        ARRAY[id] as path_ids,
        name as full_path
    FROM folders 
    WHERE parent_id IS NULL AND deleted_at IS NULL
    
    UNION ALL
    
    -- Recursive case: child folders
    SELECT 
        f.id,
        f.name,
        f.parent_id,
        f.user_id,
        f.path,
        ft.depth + 1,
        ft.path_ids || f.id,
        ft.full_path || '/' || f.name
    FROM folders f
    INNER JOIN folder_tree ft ON f.parent_id = ft.id
    WHERE f.deleted_at IS NULL AND ft.depth < 10 -- Prevent infinite recursion
)
SELECT * FROM folder_tree;

-- Indexes on materialized views
CREATE INDEX IF NOT EXISTS idx_folder_hierarchy_user_id ON folder_hierarchy(user_id);
CREATE INDEX IF NOT EXISTS idx_folder_hierarchy_parent_id ON folder_hierarchy(parent_id);
CREATE INDEX IF NOT EXISTS idx_folder_hierarchy_depth ON folder_hierarchy(depth);

-- =============================================
-- QUERY OPTIMIZATION FUNCTIONS
-- =============================================

-- Function to get user notes with pagination and search
CREATE OR REPLACE FUNCTION get_user_notes(
    p_user_id UUID,
    p_folder_id UUID DEFAULT NULL,
    p_search_query TEXT DEFAULT NULL,
    p_limit INTEGER DEFAULT 50,
    p_offset INTEGER DEFAULT 0,
    p_sort_by TEXT DEFAULT 'updated_at',
    p_sort_order TEXT DEFAULT 'DESC'
)
RETURNS TABLE(
    id UUID,
    title TEXT,
    content TEXT,
    folder_id UUID,
    version INTEGER,
    created_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ,
    size_bytes INTEGER
) AS $$
DECLARE
    sort_column TEXT;
    search_condition TEXT;
BEGIN
    -- Validate sort parameters
    sort_column := CASE 
        WHEN p_sort_by IN ('created_at', 'updated_at', 'title') THEN p_sort_by
        ELSE 'updated_at'
    END;
    
    -- Build search condition
    IF p_search_query IS NOT NULL AND LENGTH(p_search_query) > 0 THEN
        search_condition := format('AND to_tsvector(''english'', title || '' '' || content) @@ plainto_tsquery(''english'', %L)', p_search_query);
    ELSE
        search_condition := '';
    END IF;
    
    -- Build and execute dynamic query
    RETURN QUERY EXECUTE format('
        SELECT n.id, n.title, n.content, n.folder_id, n.version, 
               n.created_at, n.updated_at, LENGTH(n.content) as size_bytes
        FROM notes n
        WHERE n.user_id = $1 
        AND n.deleted_at IS NULL
        AND ($2 IS NULL OR n.folder_id = $2)
        %s
        ORDER BY %I %s
        LIMIT $3 OFFSET $4',
        search_condition, sort_column, 
        CASE WHEN upper(p_sort_order) = 'DESC' THEN 'DESC' ELSE 'ASC' END
    ) USING p_user_id, p_folder_id, p_limit, p_offset;
END;
$$ LANGUAGE plpgsql STABLE;

-- Function to get folder tree for user
CREATE OR REPLACE FUNCTION get_user_folder_tree(p_user_id UUID)
RETURNS TABLE(
    id UUID,
    name TEXT,
    parent_id UUID,
    depth INTEGER,
    path_ids UUID[],
    full_path TEXT,
    note_count BIGINT
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        fh.id,
        fh.name,
        fh.parent_id,
        fh.depth,
        fh.path_ids,
        fh.full_path,
        COALESCE(nc.note_count, 0) as note_count
    FROM folder_hierarchy fh
    LEFT JOIN (
        SELECT folder_id, COUNT(*) as note_count
        FROM notes
        WHERE user_id = p_user_id AND deleted_at IS NULL
        GROUP BY folder_id
    ) nc ON fh.id = nc.folder_id
    WHERE fh.user_id = p_user_id
    ORDER BY fh.depth, fh.name;
END;
$$ LANGUAGE plpgsql STABLE;

-- =============================================
-- PERFORMANCE MONITORING
-- =============================================

-- View for monitoring slow queries
CREATE OR REPLACE VIEW slow_queries AS
SELECT 
    query,
    calls,
    total_time,
    mean_time,
    stddev_time,
    rows,
    100.0 * shared_blks_hit / nullif(shared_blks_hit + shared_blks_read, 0) AS hit_percent
FROM pg_stat_statements
WHERE mean_time > 100 -- Queries slower than 100ms
ORDER BY mean_time DESC;

-- View for monitoring table statistics
CREATE OR REPLACE VIEW table_stats AS
SELECT 
    schemaname,
    tablename,
    attname,
    n_distinct,
    correlation,
    null_frac,
    avg_width
FROM pg_stats
WHERE schemaname = 'public'
ORDER BY tablename, attname;

-- =============================================
-- MAINTENANCE FUNCTIONS
-- =============================================

-- Function to refresh materialized views
CREATE OR REPLACE FUNCTION refresh_materialized_views()
RETURNS void AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY user_stats;
    REFRESH MATERIALIZED VIEW CONCURRENTLY folder_hierarchy;
    
    -- Log the refresh
    INSERT INTO maintenance_log (operation, completed_at) 
    VALUES ('refresh_materialized_views', NOW());
END;
$$ LANGUAGE plpgsql;

-- Function to update table statistics
CREATE OR REPLACE FUNCTION update_table_statistics()
RETURNS void AS $$
BEGIN
    ANALYZE users;
    ANALYZE notes;
    ANALYZE folders;
    
    -- Log the analyze
    INSERT INTO maintenance_log (operation, completed_at) 
    VALUES ('update_table_statistics', NOW());
END;
$$ LANGUAGE plpgsql;

-- Create maintenance log table
CREATE TABLE IF NOT EXISTS maintenance_log (
    id SERIAL PRIMARY KEY,
    operation TEXT NOT NULL,
    completed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    duration INTERVAL GENERATED ALWAYS AS (completed_at - completed_at) STORED
);

-- =============================================
-- AUTOMATED MAINTENANCE
-- =============================================

-- Function to perform routine maintenance
CREATE OR REPLACE FUNCTION perform_maintenance()
RETURNS void AS $$
BEGIN
    -- Update statistics
    PERFORM update_table_statistics();
    
    -- Refresh materialized views
    PERFORM refresh_materialized_views();
    
    -- Reindex if needed (based on table bloat)
    PERFORM reindex_if_needed();
    
    -- Vacuum analyze critical tables
    PERFORM vacuum_critical_tables();
END;
$$ LANGUAGE plpgsql;

-- Function to reindex tables if fragmentation is high
CREATE OR REPLACE FUNCTION reindex_if_needed()
RETURNS void AS $$
DECLARE
    table_record RECORD;
BEGIN
    FOR table_record IN 
        SELECT schemaname, tablename 
        FROM pg_stat_user_tables 
        WHERE schemaname = 'public' 
        AND (n_tup_ins + n_tup_upd + n_tup_del) > 10000
    LOOP
        EXECUTE format('REINDEX TABLE %I.%I', table_record.schemaname, table_record.tablename);
    END LOOP;
    
    INSERT INTO maintenance_log (operation, completed_at) 
    VALUES ('reindex_if_needed', NOW());
END;
$$ LANGUAGE plpgsql;

-- Function to vacuum critical tables
CREATE OR REPLACE FUNCTION vacuum_critical_tables()
RETURNS void AS $$
BEGIN
    VACUUM ANALYZE users;
    VACUUM ANALYZE notes;
    VACUUM ANALYZE folders;
    
    INSERT INTO maintenance_log (operation, completed_at) 
    VALUES ('vacuum_critical_tables', NOW());
END;
$$ LANGUAGE plpgsql;

-- =============================================
-- PERFORMANCE CONSTRAINTS
-- =============================================

-- Constraint to limit note size to 1MB
ALTER TABLE notes ADD CONSTRAINT IF NOT EXISTS chk_note_size 
CHECK (LENGTH(content) <= 1048576);

-- Constraint to limit folder depth
CREATE OR REPLACE FUNCTION check_folder_depth()
RETURNS TRIGGER AS $$
BEGIN
    IF (SELECT COUNT(*) FROM folder_hierarchy WHERE id = NEW.parent_id AND depth >= 9) > 0 THEN
        RAISE EXCEPTION 'Folder depth cannot exceed 10 levels';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER IF NOT EXISTS trigger_check_folder_depth
    BEFORE INSERT OR UPDATE ON folders
    FOR EACH ROW EXECUTE FUNCTION check_folder_depth();

-- =============================================
-- CONNECTION POOLING SETTINGS
-- =============================================

-- Recommended PostgreSQL settings for performance
-- Add these to postgresql.conf:

/*
# Memory settings
shared_buffers = 256MB                  # 1/4 of available RAM
effective_cache_size = 1GB              # 3/4 of available RAM
work_mem = 4MB                          # For sorting and hashing
maintenance_work_mem = 64MB             # For maintenance operations

# Connection settings
max_connections = 100                   # Adjust based on application needs
shared_preload_libraries = 'pg_stat_statements'

# Query planner settings
random_page_cost = 1.1                  # For SSD storage
effective_io_concurrency = 200          # For SSD storage

# Write-ahead logging
wal_buffers = 16MB
checkpoint_completion_target = 0.7
wal_writer_delay = 200ms

# Background writer
bgwriter_delay = 200ms
bgwriter_lru_maxpages = 100
bgwriter_lru_multiplier = 2.0

# Vacuum settings
autovacuum_vacuum_scale_factor = 0.1
autovacuum_analyze_scale_factor = 0.05
autovacuum_naptime = 15s

# Logging for performance monitoring
log_min_duration_statement = 100        # Log queries > 100ms
log_checkpoints = on
log_connections = on
log_disconnections = on
log_lock_waits = on
*/

-- =============================================
-- QUERY PLAN HELPERS
-- =============================================

-- Function to explain query plans for debugging
CREATE OR REPLACE FUNCTION explain_note_query(
    p_user_id UUID,
    p_folder_id UUID DEFAULT NULL,
    p_search_query TEXT DEFAULT NULL
)
RETURNS TABLE(plan_line TEXT) AS $$
BEGIN
    RETURN QUERY
    SELECT * FROM (
        EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT)
        SELECT id, title, updated_at 
        FROM notes 
        WHERE user_id = p_user_id
        AND (p_folder_id IS NULL OR folder_id = p_folder_id)
        AND (p_search_query IS NULL OR 
             to_tsvector('english', title || ' ' || content) @@ plainto_tsquery('english', p_search_query))
        ORDER BY updated_at DESC
        LIMIT 50
    ) AS plan_table;
END;
$$ LANGUAGE plpgsql;

-- =============================================
-- CLEANUP SCRIPT
-- =============================================

-- Schedule this to run periodically (e.g., daily via cron)
CREATE OR REPLACE FUNCTION daily_maintenance()
RETURNS void AS $$
BEGIN
    -- Clean up old maintenance logs (keep only last 30 days)
    DELETE FROM maintenance_log 
    WHERE completed_at < NOW() - INTERVAL '30 days';
    
    -- Update statistics on main tables
    PERFORM update_table_statistics();
    
    -- Refresh materialized views during low traffic hours
    PERFORM refresh_materialized_views();
    
    -- Log completion
    INSERT INTO maintenance_log (operation, completed_at) 
    VALUES ('daily_maintenance', NOW());
END;
$$ LANGUAGE plpgsql;