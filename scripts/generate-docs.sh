#!/bin/bash

# API Documentation Generation Script
# Generates comprehensive API documentation from OpenAPI specification

set -e

echo "🔧 Generating API Documentation"
echo "==============================="

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1"
}

success() {
    echo -e "${GREEN}✓${NC} $1"
}

# Create docs directory
DOCS_DIR="docs/api"
mkdir -p $DOCS_DIR

# Check if swagger-codegen is available
check_dependencies() {
    log "Checking documentation generation dependencies..."
    
    # Check for swagger-codegen-cli
    if ! command -v swagger-codegen &> /dev/null; then
        echo "swagger-codegen not found. Installing..."
        
        # Try to install via npm if available
        if command -v npm &> /dev/null; then
            npm install -g @apidevtools/swagger-cli
            success "swagger-cli installed via npm"
        else
            echo "Please install swagger-codegen-cli manually:"
            echo "npm install -g @apidevtools/swagger-cli"
            echo "or download from: https://github.com/swagger-api/swagger-codegen"
            exit 1
        fi
    else
        success "swagger-codegen found"
    fi
    
    # Check for redoc-cli for alternative documentation
    if ! command -v redoc-cli &> /dev/null; then
        if command -v npm &> /dev/null; then
            echo "Installing redoc-cli for enhanced documentation..."
            npm install -g redoc-cli
            success "redoc-cli installed"
        fi
    fi
}

# Generate HTML documentation
generate_html_docs() {
    log "Generating HTML API documentation..."
    
    cd backend
    
    # Generate Swagger UI documentation
    if command -v swagger-codegen &> /dev/null; then
        swagger-codegen generate \
            -i api-spec/openapi.yaml \
            -l html2 \
            -o ../docs/api/swagger-ui \
            --additional-properties=appName="Fastest Note App API",appDescription="High-performance note-taking API"
        
        success "Swagger UI documentation generated in docs/api/swagger-ui/"
    fi
    
    # Generate ReDoc documentation (alternative, more modern)
    if command -v redoc-cli &> /dev/null; then
        redoc-cli build api-spec/openapi.yaml \
            --output ../docs/api/redoc.html \
            --title "Fastest Note App API Documentation" \
            --options.theme.colors.primary.main=#2563eb
        
        success "ReDoc documentation generated at docs/api/redoc.html"
    fi
    
    cd ..
}

# Generate markdown documentation
generate_markdown_docs() {
    log "Generating Markdown API documentation..."
    
    # Create comprehensive markdown documentation
    cat > "$DOCS_DIR/README.md" << 'EOF'
# Fastest Note App API Documentation

## Overview

The Fastest Note App API is a high-performance REST API designed to provide sub-200ms response times for note-taking operations. This API powers a note-taking application that aims to be faster than Notion.

## Key Features

- **High Performance**: API response times < 200ms (95th percentile)
- **Real-time Sync**: WebSocket support with < 50ms message delivery
- **Hierarchical Organization**: Up to 10 levels of folder nesting
- **Full-text Search**: < 100ms search across all notes
- **Offline Support**: Conflict resolution for offline edits
- **Data Limits**: 1MB max note size, 1000 items per folder

## Base URL

- **Development**: `http://localhost:3001/api/v1`
- **Production**: `https://api.fastest-notes.com/v1`

## Authentication

All API endpoints (except registration, login, and health check) require JWT authentication.

### Getting Started

1. **Register a new account**:
```bash
curl -X POST http://localhost:3001/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com", "password": "SecurePass123!"}'
```

2. **Login to get tokens**:
```bash
curl -X POST http://localhost:3001/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com", "password": "SecurePass123!"}'
```

3. **Use the access token in requests**:
```bash
curl -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  http://localhost:3001/api/v1/notes
```

## Rate Limits

- **Authenticated requests**: 1000 requests per minute
- **Registration/Login**: 10 requests per minute
- **Search**: 100 requests per minute

## Performance Guarantees

| Operation | Target Response Time |
|-----------|---------------------|
| Note CRUD operations | < 200ms |
| Folder operations | < 200ms |
| Search queries | < 100ms |
| WebSocket messages | < 50ms |

## Error Handling

The API uses standard HTTP status codes and returns consistent error responses:

```json
{
  "error": "validation_failed",
  "message": "Request validation failed",
  "validation_errors": {
    "title": ["Title is required"],
    "content": ["Content exceeds 1MB limit"]
  }
}
```

## Data Limits

- **Note content**: Maximum 1MB per note
- **Note title**: Maximum 255 characters
- **Folder depth**: Maximum 10 levels
- **Items per folder**: Maximum 1000 (notes + subfolders combined)
- **Folder name**: Maximum 255 characters

## WebSocket Real-time Updates

Connect to `ws://localhost:3001/ws?token=YOUR_JWT_TOKEN` for real-time updates.

### Message Types

- `subscribe_note`: Subscribe to note updates
- `note_updated`: Real-time note update notification
- `note_created`: New note notification
- `note_deleted`: Note deletion notification

### Example WebSocket Usage

```javascript
const ws = new WebSocket('ws://localhost:3001/ws?token=' + accessToken);

// Subscribe to note updates
ws.send(JSON.stringify({
  type: 'subscribe_note',
  id: 'sub_001',
  data: { note_id: 'note-uuid-here' }
}));

// Handle real-time updates
ws.onmessage = (event) => {
  const message = JSON.parse(event.data);
  console.log('Real-time update:', message);
};
```

## SDK and Client Libraries

- **JavaScript/TypeScript**: Available in the frontend implementation
- **Rust**: Native client using reqwest
- **Python**: Generated client available
- **cURL**: All examples provided in cURL format

## OpenAPI Specification

The complete OpenAPI 3.0 specification is available at:
- **YAML**: `backend/api-spec/openapi.yaml`
- **Interactive Docs**: `http://localhost:3001/docs` (when server is running)
- **ReDoc**: Available in this docs folder

## Testing

Comprehensive test suite including:
- Unit tests for all business logic
- Integration tests for API endpoints
- Performance benchmarks validating response time targets
- End-to-end user journey tests

Run tests with:
```bash
cd backend && cargo test
```

## Support

- **Issues**: https://github.com/fastest-note-app/issues
- **Email**: support@fastest-notes.com
- **Documentation**: This README and OpenAPI spec

## License

MIT License - see LICENSE file for details.
EOF
    
    success "Markdown documentation created at $DOCS_DIR/README.md"
}

# Generate code examples
generate_examples() {
    log "Generating API usage examples..."
    
    cat > "$DOCS_DIR/examples.md" << 'EOF'
# API Usage Examples

This document provides practical examples for using the Fastest Note App API.

## Authentication Examples

### Register New User
```bash
curl -X POST http://localhost:3001/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "newuser@example.com",
    "password": "SecurePassword123!"
  }'
```

**Response:**
```json
{
  "access_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "refresh_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "expires_in": 3600,
  "user": {
    "id": "123e4567-e89b-12d3-a456-426614174000",
    "email": "newuser@example.com",
    "created_at": "2023-12-07T10:30:00Z"
  }
}
```

### Login Existing User
```bash
curl -X POST http://localhost:3001/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "newuser@example.com",
    "password": "SecurePassword123!"
  }'
```

## Notes Examples

### Create Note
```bash
curl -X POST http://localhost:3001/api/v1/notes \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "title": "Meeting Notes",
    "content": "Discussed project timeline and deliverables..."
  }'
```

### List Notes with Search
```bash
# Get all notes
curl -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  "http://localhost:3001/api/v1/notes"

# Search notes
curl -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  "http://localhost:3001/api/v1/notes?search=meeting&limit=20"

# Get notes from specific folder
curl -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  "http://localhost:3001/api/v1/notes?folder_id=folder-uuid-here"
```

### Update Note with Optimistic Locking
```bash
curl -X PUT http://localhost:3001/api/v1/notes/note-uuid-here \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "title": "Updated Meeting Notes",
    "content": "Updated content with additional notes...",
    "version": 2
  }'
```

### Move Note to Folder
```bash
curl -X POST http://localhost:3001/api/v1/notes/note-uuid-here/move \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "folder_id": "target-folder-uuid"
  }'
```

## Folders Examples

### Create Root Folder
```bash
curl -X POST http://localhost:3001/api/v1/folders \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Work Projects"
  }'
```

### Create Subfolder
```bash
curl -X POST http://localhost:3001/api/v1/folders \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Project Alpha",
    "parent_folder_id": "parent-folder-uuid"
  }'
```

### Get Folder with Contents
```bash
curl -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  "http://localhost:3001/api/v1/folders/folder-uuid-here?include_contents=true"
```

## JavaScript/TypeScript Examples

### Using Fetch API

```javascript
// Configuration
const API_BASE = 'http://localhost:3001/api/v1';
let accessToken = localStorage.getItem('access_token');

// Helper function for authenticated requests
async function apiRequest(endpoint, options = {}) {
  const response = await fetch(`${API_BASE}${endpoint}`, {
    headers: {
      'Authorization': `Bearer ${accessToken}`,
      'Content-Type': 'application/json',
      ...options.headers
    },
    ...options
  });
  
  if (!response.ok) {
    throw new Error(`API request failed: ${response.statusText}`);
  }
  
  return response.json();
}

// Create a note
async function createNote(title, content, folderId = null) {
  return apiRequest('/notes', {
    method: 'POST',
    body: JSON.stringify({ title, content, folder_id: folderId })
  });
}

// Search notes
async function searchNotes(query) {
  return apiRequest(`/notes?search=${encodeURIComponent(query)}`);
}

// Usage
try {
  const note = await createNote('My Note', 'Note content here');
  console.log('Created note:', note);
  
  const results = await searchNotes('meeting notes');
  console.log('Search results:', results);
} catch (error) {
  console.error('API error:', error);
}
```

### Using Axios

```javascript
import axios from 'axios';

// Configure axios instance
const api = axios.create({
  baseURL: 'http://localhost:3001/api/v1',
  headers: {
    'Authorization': `Bearer ${accessToken}`
  }
});

// Create folder hierarchy
async function createFolderHierarchy() {
  try {
    // Create root folder
    const rootFolder = await api.post('/folders', {
      name: 'Projects'
    });
    
    // Create subfolder
    const subFolder = await api.post('/folders', {
      name: 'Web Development',
      parent_folder_id: rootFolder.data.id
    });
    
    // Create note in subfolder
    const note = await api.post('/notes', {
      title: 'Project Requirements',
      content: 'List of requirements for the web project...',
      folder_id: subFolder.data.id
    });
    
    return { rootFolder: rootFolder.data, subFolder: subFolder.data, note: note.data };
  } catch (error) {
    console.error('Error creating hierarchy:', error.response.data);
    throw error;
  }
}
```

## WebSocket Examples

### JavaScript WebSocket Client

```javascript
class NotesWebSocketClient {
  constructor(accessToken) {
    this.accessToken = accessToken;
    this.ws = null;
    this.subscriptions = new Map();
  }
  
  connect() {
    this.ws = new WebSocket(`ws://localhost:3001/ws?token=${this.accessToken}`);
    
    this.ws.onopen = () => {
      console.log('WebSocket connected');
    };
    
    this.ws.onmessage = (event) => {
      const message = JSON.parse(event.data);
      this.handleMessage(message);
    };
    
    this.ws.onclose = () => {
      console.log('WebSocket disconnected');
      // Implement reconnection logic
      setTimeout(() => this.connect(), 1000);
    };
  }
  
  subscribeToNote(noteId, callback) {
    const subscriptionId = `note_${noteId}`;
    this.subscriptions.set(subscriptionId, callback);
    
    this.ws.send(JSON.stringify({
      type: 'subscribe_note',
      id: subscriptionId,
      data: { note_id: noteId }
    }));
  }
  
  handleMessage(message) {
    if (message.type === 'note_updated') {
      const callback = this.subscriptions.get(`note_${message.data.note_id}`);
      if (callback) {
        callback(message.data);
      }
    }
  }
}

// Usage
const wsClient = new NotesWebSocketClient(accessToken);
wsClient.connect();

// Subscribe to note updates
wsClient.subscribeToNote('note-uuid-here', (noteData) => {
  console.log('Note updated:', noteData);
  // Update UI with new note data
});
```

## Performance Testing Examples

### Load Testing with Artillery

```yaml
# artillery-config.yml
config:
  target: 'http://localhost:3001'
  phases:
    - duration: 60
      arrivalRate: 10
  variables:
    access_token: 'YOUR_ACCESS_TOKEN_HERE'

scenarios:
  - name: 'Note operations'
    weight: 70
    flow:
      - post:
          url: '/api/v1/notes'
          headers:
            Authorization: 'Bearer {{ access_token }}'
            Content-Type: 'application/json'
          json:
            title: 'Load Test Note {{ $randomInt(1, 1000) }}'
            content: 'Content for load testing'
      - get:
          url: '/api/v1/notes'
          headers:
            Authorization: 'Bearer {{ access_token }}'

  - name: 'Search operations'
    weight: 30
    flow:
      - get:
          url: '/api/v1/notes?search=test'
          headers:
            Authorization: 'Bearer {{ access_token }}'
```

Run with: `artillery run artillery-config.yml`

### Benchmark with Apache Bench

```bash
# Test note creation performance
ab -n 1000 -c 10 \
   -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
   -H "Content-Type: application/json" \
   -p note-data.json \
   http://localhost:3001/api/v1/notes

# Test note retrieval performance  
ab -n 1000 -c 10 \
   -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
   http://localhost:3001/api/v1/notes
```

Where `note-data.json` contains:
```json
{
  "title": "Benchmark Note",
  "content": "This is a benchmark test note."
}
```

## Error Handling Examples

### Handling Validation Errors

```javascript
async function createNoteWithValidation(title, content) {
  try {
    const response = await fetch('http://localhost:3001/api/v1/notes', {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${accessToken}`,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({ title, content })
    });
    
    if (!response.ok) {
      const errorData = await response.json();
      
      if (response.status === 422) {
        // Handle validation errors
        console.error('Validation errors:', errorData.validation_errors);
        
        // Display specific field errors
        Object.entries(errorData.validation_errors).forEach(([field, errors]) => {
          errors.forEach(error => {
            console.error(`${field}: ${error}`);
          });
        });
      } else if (response.status === 401) {
        // Handle authentication errors
        console.error('Authentication failed - redirecting to login');
        // Redirect to login page
      }
      
      throw new Error(errorData.message);
    }
    
    return response.json();
  } catch (error) {
    console.error('Failed to create note:', error.message);
    throw error;
  }
}
```

### Handling Version Conflicts

```javascript
async function updateNoteWithConflictHandling(noteId, updates) {
  try {
    const response = await fetch(`http://localhost:3001/api/v1/notes/${noteId}`, {
      method: 'PUT',
      headers: {
        'Authorization': `Bearer ${accessToken}`,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(updates)
    });
    
    if (response.status === 409) {
      // Version conflict - note was modified by another user
      const conflictData = await response.json();
      console.warn('Version conflict detected');
      
      // Show conflict resolution UI
      const userChoice = await showConflictResolution(conflictData.current_data, updates);
      
      if (userChoice === 'overwrite') {
        // Retry with current version
        updates.version = conflictData.current_version;
        return updateNoteWithConflictHandling(noteId, updates);
      } else if (userChoice === 'merge') {
        // Implement merge logic
        const mergedUpdates = mergeNoteData(conflictData.current_data, updates);
        mergedUpdates.version = conflictData.current_version;
        return updateNoteWithConflictHandling(noteId, mergedUpdates);
      }
      // User chose to cancel - return current data
      return conflictData.current_data;
    }
    
    return response.json();
  } catch (error) {
    console.error('Failed to update note:', error);
    throw error;
  }
}
```

These examples demonstrate the key patterns for integrating with the Fastest Note App API while maintaining high performance and proper error handling.
EOF
    
    success "API examples created at $DOCS_DIR/examples.md"
}

# Generate performance documentation
generate_performance_docs() {
    log "Generating performance documentation..."
    
    cat > "$DOCS_DIR/performance.md" << 'EOF'
# Performance Documentation

## Performance Targets

The Fastest Note App API is designed with strict performance requirements:

| Operation | Target | Measurement |
|-----------|---------|------------|
| Note CRUD operations | < 200ms | 95th percentile response time |
| Folder operations | < 200ms | 95th percentile response time |
| Search queries | < 100ms | 95th percentile response time |
| WebSocket messages | < 50ms | End-to-end delivery time |
| Authentication | < 200ms | Login/register response time |

## Performance Features

### Database Optimization
- **Connection Pooling**: deadpool-postgres with optimized pool size
- **Prepared Statements**: All queries use prepared statements
- **Indexes**: Strategic indexing on frequently queried fields
- **Query Optimization**: Analyzed and optimized query execution plans

### Caching Strategy
- **Redis Cache**: Frequently accessed data cached in Redis
- **Cache Invalidation**: Smart cache invalidation on data updates
- **Cache Warming**: Pre-populate cache for common queries
- **TTL Management**: Appropriate time-to-live settings

### API Optimizations
- **Async/Await**: Non-blocking I/O operations throughout
- **Connection Reuse**: HTTP connection pooling
- **Compression**: GZIP compression for large responses
- **Pagination**: Efficient pagination to limit response sizes

### Real-time Performance
- **WebSocket Optimization**: Minimal overhead for real-time updates
- **Message Queuing**: Efficient message delivery system
- **Connection Management**: Smart WebSocket connection handling

## Performance Monitoring

### Metrics Collection
```rust
// Example metrics collection in Rust
use std::time::Instant;

async fn timed_operation<F, R>(operation: F, operation_name: &str) -> R 
where 
    F: Future<Output = R>,
{
    let start = Instant::now();
    let result = operation.await;
    let duration = start.elapsed();
    
    // Log performance metric
    tracing::info!(
        operation = operation_name,
        duration_ms = duration.as_millis(),
        "Operation completed"
    );
    
    result
}
```

### Performance Testing
Run comprehensive performance tests:

```bash
# Backend performance benchmarks
cd backend && cargo test --release performance_

# Load testing with artillery
artillery run performance-tests/load-test.yml

# Database performance
cd backend && cargo bench
```

### Performance Alerts
Monitor these key metrics:
- Average response time > 150ms
- 95th percentile response time > 200ms
- Error rate > 1%
- Database connection pool utilization > 80%
- Redis hit rate < 90%

## Optimization Techniques

### Database Query Optimization
```sql
-- Example optimized queries with proper indexing
CREATE INDEX CONCURRENTLY idx_notes_user_updated 
ON notes(user_id, updated_at DESC);

CREATE INDEX CONCURRENTLY idx_notes_search 
ON notes USING gin(to_tsvector('english', title || ' ' || content));

-- Query with proper index usage
SELECT id, title, updated_at 
FROM notes 
WHERE user_id = $1 
ORDER BY updated_at DESC 
LIMIT 50;
```

### Rust Performance Optimizations
```rust
// Use efficient data structures
use std::collections::HashMap;
use tokio::sync::RwLock;

// Minimize allocations
#[derive(Clone)]
pub struct CachedNote {
    pub id: Uuid,
    pub title: Arc<str>,  // Arc for cheap cloning
    pub updated_at: DateTime<Utc>,
}

// Use appropriate concurrency primitives
type NoteCache = Arc<RwLock<HashMap<Uuid, CachedNote>>>;
```

### Frontend Optimizations
- **Virtual Scrolling**: Handle large lists efficiently
- **Debounced Search**: Reduce API calls during typing
- **Optimistic Updates**: Update UI before API confirmation
- **Bundle Splitting**: Load code on demand
- **Service Workers**: Cache API responses

## Performance Best Practices

### API Design
1. **Pagination**: Always paginate large result sets
2. **Field Selection**: Allow clients to specify required fields
3. **Bulk Operations**: Provide batch endpoints for multiple items
4. **Compression**: Use GZIP for responses > 1KB
5. **HTTP Caching**: Implement appropriate cache headers

### Database Best Practices
1. **Connection Pooling**: Size pools appropriately
2. **Query Analysis**: Regular EXPLAIN ANALYZE on slow queries
3. **Index Maintenance**: Monitor and maintain database indexes
4. **Connection Limits**: Set appropriate connection limits
5. **Read Replicas**: Use read replicas for scaling reads

### Caching Best Practices
1. **Cache Keys**: Use consistent, hierarchical cache keys
2. **Invalidation**: Implement proper cache invalidation
3. **TTL Strategy**: Set appropriate expiration times
4. **Cache Warming**: Pre-populate frequently accessed data
5. **Monitoring**: Monitor cache hit rates and eviction rates

## Troubleshooting Performance Issues

### Database Performance
```bash
# Check slow queries
SELECT query, mean_time, calls 
FROM pg_stat_statements 
ORDER BY mean_time DESC LIMIT 10;

# Check index usage
SELECT schemaname, tablename, indexname, idx_scan, idx_tup_read, idx_tup_fetch
FROM pg_stat_user_indexes
WHERE idx_scan = 0;
```

### Application Performance
```bash
# Check CPU and memory usage
top -p $(pgrep fastest-note)

# Check database connections
ss -tuln | grep 5432

# Check Redis performance
redis-cli info stats
```

### Network Performance
```bash
# Test API response times
curl -w "@curl-format.txt" -o /dev/null -s "http://localhost:3001/api/v1/notes"

# Where curl-format.txt contains:
#     time_namelookup:  %{time_namelookup}s\n
#        time_connect:  %{time_connect}s\n
#     time_appconnect:  %{time_appconnect}s\n
#    time_pretransfer:  %{time_pretransfer}s\n
#       time_redirect:  %{time_redirect}s\n
#  time_starttransfer:  %{time_starttransfer}s\n
#                     ----------\n
#          time_total:  %{time_total}s\n
```

## Scaling Considerations

### Horizontal Scaling
- **Load Balancing**: Use nginx or HAProxy for load balancing
- **Database Sharding**: Shard by user_id for large datasets
- **Microservices**: Split into focused microservices if needed
- **CDN**: Use CDN for static assets and cached API responses

### Vertical Scaling
- **CPU Optimization**: Profile and optimize CPU-intensive operations
- **Memory Management**: Optimize memory usage and garbage collection
- **Disk I/O**: Use SSDs and optimize disk access patterns
- **Network**: Optimize network configuration and bandwidth

This performance documentation ensures the API meets its ambitious speed targets while providing guidance for maintaining and improving performance.
EOF
    
    success "Performance documentation created at $DOCS_DIR/performance.md"
}

# Main execution
main() {
    log "Starting API documentation generation..."
    
    # check_dependencies
    generate_html_docs
    generate_markdown_docs
    generate_examples
    generate_performance_docs
    
    # Create index file
    cat > "$DOCS_DIR/index.html" << 'EOF'
<!DOCTYPE html>
<html>
<head>
    <title>Fastest Note App API Documentation</title>
    <meta charset="utf-8">
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; }
        .header { text-align: center; margin-bottom: 40px; }
        .nav { display: flex; gap: 20px; justify-content: center; margin-bottom: 30px; }
        .nav a { padding: 10px 20px; background: #2563eb; color: white; text-decoration: none; border-radius: 5px; }
        .nav a:hover { background: #1d4ed8; }
    </style>
</head>
<body>
    <div class="header">
        <h1>🚀 Fastest Note App API Documentation</h1>
        <p>High-performance note-taking API with sub-200ms response times</p>
    </div>
    
    <div class="nav">
        <a href="redoc.html">Interactive API Docs</a>
        <a href="README.md">Getting Started</a>
        <a href="examples.md">Code Examples</a>
        <a href="performance.md">Performance Guide</a>
    </div>
    
    <h2>Quick Links</h2>
    <ul>
        <li><strong>OpenAPI Spec</strong>: <a href="../backend/api-spec/openapi.yaml">openapi.yaml</a></li>
        <li><strong>Performance Targets</strong>: &lt;200ms API, &lt;100ms search, &lt;50ms WebSocket</li>
        <li><strong>Base URL</strong>: <code>http://localhost:3001/api/v1</code></li>
        <li><strong>Authentication</strong>: JWT Bearer tokens</li>
    </ul>
    
    <h2>Key Features</h2>
    <ul>
        <li>✅ Sub-200ms API response times</li>
        <li>✅ Real-time WebSocket synchronization</li>
        <li>✅ Hierarchical folder organization (10 levels)</li>
        <li>✅ Full-text search (&lt;100ms)</li>
        <li>✅ Optimistic locking for conflict resolution</li>
        <li>✅ 1MB note size limit, 1000 items per folder</li>
    </ul>
    
    <footer style="margin-top: 40px; text-align: center; color: #666;">
        <p>Generated on $(date) | API Version 1.0.0</p>
    </footer>
</body>
</html>
EOF
    
    success "Documentation index created at $DOCS_DIR/index.html"
    
    echo ""
    success "🎉 API documentation generation completed!"
    echo ""
    echo "Generated documentation:"
    echo "  📁 docs/api/README.md - Getting started guide"
    echo "  📁 docs/api/examples.md - Code examples"  
    echo "  📁 docs/api/performance.md - Performance guide"
    echo "  📁 docs/api/index.html - Documentation portal"
    if [ -f "$DOCS_DIR/redoc.html" ]; then
        echo "  📁 docs/api/redoc.html - Interactive API documentation"
    fi
    echo ""
    echo "📖 Open docs/api/index.html in your browser to view the documentation portal"
}

# Execute main function
main "$@"