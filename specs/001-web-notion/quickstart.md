# Quickstart Guide: Fast Note-Taking App

## Overview
This guide walks through the core user journey to validate the implementation meets specification requirements. Each step corresponds to functional requirements from the spec.

## Prerequisites
- Backend server running on `http://localhost:3001`
- Frontend development server on `http://localhost:3000` 
- PostgreSQL database with schema initialized
- Redis server for caching

## Test User Journey

### 1. User Registration & Authentication (FR-008)

**Action**: Register new user account
```bash
# Test with API directly
curl -X POST http://localhost:3001/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "password": "SecurePass123!"
  }'
```

**Expected Result**: 
- Receives JWT access token and refresh token
- User can access protected endpoints
- Password is securely hashed (never stored in plaintext)

**UI Test**: 
1. Navigate to registration page
2. Enter email and password
3. Submit form
4. Should redirect to main app interface

### 2. Create First Note (FR-001)

**Performance Target**: Note editor appears within 200ms

**Action**: Create a new note at root level
```bash
# Test API performance
time curl -X POST http://localhost:3001/api/v1/notes \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{
    "title": "My First Note",
    "content": "This is my first note content."
  }'
```

**Expected Result**:
- API response time < 200ms
- Note created with unique ID
- Note visible in UI immediately

**UI Test**:
1. Click "New Note" button
2. Measure time until editor is ready for input
3. Type title and content
4. Note auto-saves without user action

### 3. Create Folder Hierarchy (FR-002)

**Action**: Create nested folder structure
```bash
# Create root folder
curl -X POST http://localhost:3001/api/v1/folders \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Work Projects"
  }'

# Create subfolder (level 2)
curl -X POST http://localhost:3001/api/v1/folders \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Project Alpha",
    "parent_folder_id": "<work_folder_id>"
  }'
```

**Expected Result**:
- Hierarchical folder structure created
- Path and level fields automatically calculated
- Folder depth limited to 10 levels (FR-013)

**UI Test**:
1. Right-click in folder area
2. Select "New Folder"  
3. Enter folder name
4. Folder appears instantly in tree view
5. Can create subfolders by right-clicking parent

### 4. Move Notes Between Folders (FR-007)

**Action**: Organize notes using drag-and-drop
```bash
# Move note to folder via API
curl -X POST http://localhost:3001/api/v1/notes/<note_id>/move \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{
    "folder_id": "<target_folder_id>"
  }'
```

**Expected Result**:
- Note moved to target folder
- Folder contents update immediately
- Move operation completes instantly

**UI Test**:
1. Drag note from one folder to another
2. Drop note in target folder
3. Note disappears from source folder
4. Note appears in target folder immediately
5. Folder counts update correctly

### 5. Real-time Synchronization (FR-003)

**Setup**: Open app in two browser windows with same account

**Action**: Edit note in first window
1. Open same note in both windows
2. Type content in window 1
3. Observe changes in window 2

**Expected Result**:
- Changes appear in real-time in window 2
- No data loss or corruption
- Operational transforms handle concurrent edits

**WebSocket Test**:
```javascript
// Connect to WebSocket
const ws = new WebSocket('ws://localhost:3001/ws?token=<jwt>');

// Subscribe to note updates
ws.send(JSON.stringify({
  type: 'subscribe_note',
  id: 'test_001',
  data: { note_id: '<note_id>' }
}));

// Make edit via API and observe WebSocket message
```

### 6. Offline Functionality (FR-009, FR-015)

**Action**: Test offline editing
1. Disconnect network (disable Wi-Fi or use browser dev tools)
2. Create new note and edit existing notes
3. Reconnect network
4. Verify changes synchronize

**Expected Result**:
- App continues working without network
- Changes saved locally in IndexedDB
- Sync queue processes operations on reconnection
- No data loss during offline period

**Storage Test**:
```javascript
// Check IndexedDB contents
const request = indexedDB.open('FastNotesDB');
request.onsuccess = (event) => {
  const db = event.target.result;
  // Verify notes and sync queue tables
};
```

### 7. Search Functionality (FR-006)

**Action**: Search across notes and folders
```bash
# Test search API
curl "http://localhost:3001/api/v1/notes?search=project%20alpha" \
  -H "Authorization: Bearer <token>"
```

**Expected Result**:
- Search returns relevant results instantly
- Full-text search covers note titles and content
- Results ranked by relevance

**UI Test**:
1. Enter search term in search box
2. Results appear as user types
3. Click result to open note
4. Search highlights match terms

### 8. Performance Validation

**Load Testing**: Simulate concurrent users
```bash
# Use tool like Apache Bench or Artillery
ab -n 1000 -c 10 -H "Authorization: Bearer <token>" \
  http://localhost:3001/api/v1/notes
```

**Targets to Validate**:
- API response time < 200ms (95th percentile)
- UI interactions < 100ms
- WebSocket message delivery < 50ms
- Search results < 100ms

### 9. Data Limits Validation (FR-010, FR-014)

**Large Note Test**:
```bash
# Create 1MB note (maximum size)
python3 -c "
import requests
import string
content = 'A' * (1024 * 1024 - 100)  # ~1MB content
response = requests.post('http://localhost:3001/api/v1/notes', 
  json={'title': 'Large Note', 'content': content},
  headers={'Authorization': 'Bearer <token>'})
print(f'Status: {response.status_code}')
"

# Try to exceed limit
content = 'A' * (1024 * 1024 + 1)  # Over 1MB - should fail
```

**Folder Limits Test**:
```bash
# Create 1000 notes in single folder (maximum)
for i in {1..1000}; do
  curl -X POST http://localhost:3001/api/v1/notes \
    -H "Authorization: Bearer <token>" \
    -H "Content-Type: application/json" \
    -d "{\"title\": \"Note $i\", \"folder_id\": \"<folder_id>\"}"
done

# 1001st note should trigger pagination
```

### 10. Conflict Resolution (FR-011)

**Action**: Simulate simultaneous edits
1. Open same note in two clients
2. Modify content in both without saving
3. Save from first client
4. Save from second client

**Expected Result**:
- Second client receives conflict notification
- User presented with merge options
- Last-write-wins applied with user notification
- No silent data loss

## Success Criteria Checklist

**Performance** (Sub-200ms targets):
- [ ] Note creation < 200ms
- [ ] Folder operations < 200ms  
- [ ] Search results < 100ms
- [ ] Real-time updates < 50ms

**Functionality**:
- [ ] User registration and login works
- [ ] Notes create, edit, delete successfully
- [ ] Folder hierarchy operations work
- [ ] Drag-and-drop note organization  
- [ ] Real-time sync between devices
- [ ] Offline editing with sync on reconnection
- [ ] Full-text search across notes
- [ ] Data size limits enforced
- [ ] Conflict resolution with user notification

**Reliability**:
- [ ] No data loss during network interruptions
- [ ] Graceful handling of concurrent edits
- [ ] Proper error messages for edge cases
- [ ] WebSocket reconnection works automatically

## Troubleshooting

**Performance Issues**:
- Check database indexes are created
- Verify Redis cache is working
- Monitor API response times in browser dev tools
- Use database query analysis for slow queries

**Sync Issues**:
- Verify WebSocket connection established
- Check browser console for sync errors
- Validate JWT token expiration handling
- Test IndexedDB storage limits

**UI Issues**:
- Clear browser cache and reload
- Check browser console for JavaScript errors
- Verify API endpoints return expected data format
- Test with browser dev tools network throttling

This quickstart validates all core requirements and provides a foundation for automated integration testing.