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
