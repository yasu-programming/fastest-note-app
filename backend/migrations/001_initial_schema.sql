-- Migration 001: Initial Schema
-- Fast Note-Taking App Database Schema
-- Based on data-model.md specifications

-- Enable UUID extension for PostgreSQL
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Users table: Authentication and user management
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    -- Constraints
    CONSTRAINT valid_email CHECK (email ~* '^[A-Za-z0-9._%-]+@[A-Za-z0-9.-]+[.][A-Za-z]+$'),
    CONSTRAINT password_not_empty CHECK (LENGTH(password_hash) >= 60) -- bcrypt hashes are 60 chars
);

-- Folders table: Hierarchical folder structure
CREATE TABLE folders (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    parent_folder_id UUID REFERENCES folders(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    path TEXT NOT NULL, -- Materialized path for fast hierarchy queries
    level INTEGER NOT NULL CHECK (level <= 10 AND level >= 0),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    -- Constraints
    CONSTRAINT folder_name_not_empty CHECK (LENGTH(TRIM(name)) > 0),
    CONSTRAINT folder_name_no_slash CHECK (name NOT LIKE '%/%'),
    CONSTRAINT valid_path CHECK (path LIKE '/%' OR path = '/'),
    
    -- Unique constraint: no duplicate folder names in same parent
    UNIQUE(user_id, parent_folder_id, name)
);

-- Notes table: Note content and metadata  
CREATE TABLE notes (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    folder_id UUID REFERENCES folders(id) ON DELETE SET NULL,
    title VARCHAR(500) NOT NULL,
    content TEXT,
    content_size INTEGER DEFAULT 0,
    version INTEGER DEFAULT 1,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    -- Constraints
    CONSTRAINT title_not_empty CHECK (LENGTH(TRIM(title)) > 0),
    CONSTRAINT content_size_limit CHECK (content_size <= 1048576), -- 1MB limit
    CONSTRAINT version_positive CHECK (version > 0)
);

-- Indexes for performance optimization

-- User lookups
CREATE INDEX idx_users_email ON users(email);

-- Folder hierarchy queries
CREATE INDEX idx_folders_user_id ON folders(user_id);
CREATE INDEX idx_folders_parent_id ON folders(parent_folder_id);
CREATE INDEX idx_folders_path ON folders(path);
CREATE INDEX idx_folders_user_path ON folders(user_id, path);

-- Note queries
CREATE INDEX idx_notes_user_id ON notes(user_id);
CREATE INDEX idx_notes_folder_id ON notes(folder_id);
CREATE INDEX idx_notes_user_folder ON notes(user_id, folder_id);
CREATE INDEX idx_notes_updated_at ON notes(updated_at DESC);

-- Full-text search on notes (GIN index for fast text search)
CREATE INDEX idx_notes_search ON notes USING GIN (to_tsvector('english', title || ' ' || COALESCE(content, '')));

-- Triggers for automatic timestamp updates

-- Update updated_at on users table
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_users_updated_at 
    BEFORE UPDATE ON users 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_folders_updated_at 
    BEFORE UPDATE ON folders 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_notes_updated_at 
    BEFORE UPDATE ON notes 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Trigger to maintain folder path and level on insert/update
CREATE OR REPLACE FUNCTION maintain_folder_path()
RETURNS TRIGGER AS $$
BEGIN
    -- Calculate path and level based on parent
    IF NEW.parent_folder_id IS NULL THEN
        -- Root folder
        NEW.path = '/' || NEW.name || '/';
        NEW.level = 0;
    ELSE
        -- Get parent path and level
        SELECT path, level + 1 
        INTO NEW.path, NEW.level
        FROM folders 
        WHERE id = NEW.parent_folder_id AND user_id = NEW.user_id;
        
        -- Check if parent exists and belongs to same user
        IF NEW.path IS NULL THEN
            RAISE EXCEPTION 'Invalid parent folder or parent belongs to different user';
        END IF;
        
        -- Construct full path  
        NEW.path = RTRIM(NEW.path, '/') || '/' || NEW.name || '/';
    END IF;
    
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER maintain_folder_path_trigger
    BEFORE INSERT OR UPDATE ON folders
    FOR EACH ROW EXECUTE FUNCTION maintain_folder_path();

-- Trigger to update content_size on notes
CREATE OR REPLACE FUNCTION update_note_content_size()
RETURNS TRIGGER AS $$
BEGIN
    NEW.content_size = COALESCE(LENGTH(NEW.content), 0);
    
    -- Enforce 1MB limit
    IF NEW.content_size > 1048576 THEN
        RAISE EXCEPTION 'Note content exceeds 1MB limit (current: % bytes)', NEW.content_size;
    END IF;
    
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_note_content_size_trigger
    BEFORE INSERT OR UPDATE ON notes
    FOR EACH ROW EXECUTE FUNCTION update_note_content_size();

-- Function to count notes in folder (for 1000 item limit)
CREATE OR REPLACE FUNCTION check_folder_note_limit()
RETURNS TRIGGER AS $$
DECLARE
    note_count INTEGER;
BEGIN
    IF TG_OP = 'INSERT' OR (TG_OP = 'UPDATE' AND OLD.folder_id IS DISTINCT FROM NEW.folder_id) THEN
        -- Count notes in target folder
        IF NEW.folder_id IS NOT NULL THEN
            SELECT COUNT(*) INTO note_count
            FROM notes 
            WHERE folder_id = NEW.folder_id AND user_id = NEW.user_id;
            
            IF note_count >= 1000 THEN
                RAISE EXCEPTION 'Folder cannot contain more than 1000 notes (current: %)', note_count;
            END IF;
        END IF;
    END IF;
    
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER check_folder_note_limit_trigger
    BEFORE INSERT OR UPDATE ON notes
    FOR EACH ROW EXECUTE FUNCTION check_folder_note_limit();

-- Initial data: Create system user for testing (optional)
-- INSERT INTO users (email, password_hash) 
-- VALUES ('test@example.com', '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewdBPj.MZ.GCy.3G'); -- "password123"

-- Performance statistics and monitoring views
CREATE VIEW folder_stats AS
SELECT 
    f.id,
    f.name,
    f.path,
    f.level,
    COUNT(n.id) as note_count,
    COUNT(sf.id) as subfolder_count
FROM folders f
LEFT JOIN notes n ON f.id = n.folder_id
LEFT JOIN folders sf ON f.id = sf.parent_folder_id
GROUP BY f.id, f.name, f.path, f.level;