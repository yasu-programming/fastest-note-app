# Phase 1: Data Model

## Core Entities

### User
```sql
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
```

**Fields**:
- `id`: Unique identifier (UUID for distributed systems)
- `email`: Authentication identifier, unique constraint
- `password_hash`: bcrypt hashed password (never store plaintext)
- `created_at`/`updated_at`: Audit timestamps

**Validation Rules**:
- Email must be valid email format
- Password minimum 8 characters with complexity requirements
- Email uniqueness enforced at database level

### Folder
```sql
CREATE TABLE folders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    parent_folder_id UUID REFERENCES folders(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    path TEXT NOT NULL, -- Materialized path for fast hierarchy queries
    level INTEGER NOT NULL CHECK (level <= 10),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    UNIQUE(user_id, parent_folder_id, name), -- No duplicate names in same folder
    INDEX idx_folders_user_path (user_id, path),
    INDEX idx_folders_parent (parent_folder_id)
);
```

**Fields**:
- `id`: Unique folder identifier
- `user_id`: Folder owner (soft multi-tenancy)
- `parent_folder_id`: Parent folder (NULL for root folders)
- `name`: Display name (255 char limit)
- `path`: Materialized path (e.g., "/root/subfolder/") for efficient hierarchy queries
- `level`: Depth in hierarchy (enforced max 10 levels)

**Validation Rules**:
- Name cannot be empty or contain `/` character
- Level must be ≤ 10 (business rule)
- Cannot create circular references in hierarchy
- Path automatically maintained on folder moves

**Relationships**:
- Many-to-one with User (owner)
- Self-referential hierarchy (parent/children)
- One-to-many with Notes

### Note
```sql
CREATE TABLE notes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    folder_id UUID REFERENCES folders(id) ON DELETE SET NULL,
    title VARCHAR(500) NOT NULL,
    content TEXT,
    content_size INTEGER DEFAULT 0,
    version INTEGER DEFAULT 1,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    CHECK (content_size <= 1048576), -- 1MB limit
    INDEX idx_notes_user_folder (user_id, folder_id),
    INDEX idx_notes_updated (updated_at),
    INDEX gin_notes_search (to_tsvector('english', title || ' ' || content))
);
```

**Fields**:
- `id`: Unique note identifier
- `user_id`: Note owner
- `folder_id`: Containing folder (NULL = root level)
- `title`: Note title (500 char limit for UI display)
- `content`: Full note content (TEXT = unlimited, but enforced at 1MB)
- `content_size`: Cached size in bytes for validation
- `version`: Optimistic locking for conflict detection

**Validation Rules**:
- Title cannot be empty
- Content size ≤ 1MB (1,048,576 bytes)
- Version increments on each update
- Title and content indexed for full-text search

**Relationships**:
- Many-to-one with User (owner)
- Many-to-one with Folder (optional - can be at root level)

## State Transitions

### Note Editing Flow
```
[Created] → [Editing] → [Saved] → [Synced]
    ↓           ↓          ↓         ↓
[Editing] ← [Conflict] ← [Sync_Failed] ← [Network_Error]
```

**States**:
- **Created**: New note, exists only locally
- **Editing**: User actively modifying content
- **Saved**: Changes persisted locally
- **Synced**: Successfully synchronized to server
- **Conflict**: Concurrent edit detected, needs resolution
- **Sync_Failed**: Network/server error during sync
- **Network_Error**: Connection lost during operation

### Folder Operations
```
[Folder_Created] → [Notes_Moved] → [Hierarchy_Updated] → [Synced]
```

## Performance Considerations

### Indexing Strategy
- **Folder hierarchy queries**: Materialized path with btree index
- **Note search**: GIN index on tsvector for full-text search
- **Recent notes**: Index on updated_at for "recently edited" queries
- **User isolation**: Compound indexes starting with user_id

### Caching Strategy
- **Session data**: Redis with 24-hour expiration
- **Folder structure**: Redis cache per user (invalidate on folder operations)
- **Recent notes**: Cache last 50 notes per user
- **Search results**: Cache search queries for 5 minutes

### Database Constraints
- Foreign key constraints ensure referential integrity
- Check constraints enforce business rules (file size, folder depth)
- Unique constraints prevent duplicate folder names in same parent

## Offline Data Model

### Client-Side Storage (IndexedDB)
```javascript
// Notes table
{
  id: string,
  title: string,
  content: string,
  folder_id: string | null,
  version: number,
  local_changes: boolean,
  last_synced: timestamp,
  created_at: timestamp,
  updated_at: timestamp
}

// Sync Queue table
{
  id: string,
  operation: 'CREATE' | 'UPDATE' | 'DELETE' | 'MOVE',
  entity_type: 'note' | 'folder',
  entity_id: string,
  payload: object,
  created_at: timestamp,
  retry_count: number
}
```

### Sync Operations
- **CREATE**: Add new note/folder
- **UPDATE**: Modify existing content
- **DELETE**: Remove note/folder
- **MOVE**: Change folder hierarchy

## Data Integrity Rules

### Cascading Behavior
- **User deletion**: All folders and notes deleted (CASCADE)
- **Folder deletion**: Notes moved to parent folder or root (SET NULL)
- **Parent folder deletion**: Child folders moved to grandparent (CASCADE with path update)

### Consistency Checks
- Folder path consistency maintained by triggers
- Note count limits enforced (max 1000 notes per folder)
- Total storage per user monitored (soft limit warnings)

### Backup Strategy
- **Point-in-time recovery**: PostgreSQL WAL archiving
- **Daily snapshots**: Full database backup
- **User data export**: JSON export API for individual users
- **Disaster recovery**: Cross-region database replication