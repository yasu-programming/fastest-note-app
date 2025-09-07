# WebSocket API Specification

## Connection

**Endpoint**: `ws://localhost:3001/ws` (dev) | `wss://api.fastnotes.app/ws` (prod)

**Authentication**: JWT token passed as query parameter or in Authorization header during handshake

```javascript
const ws = new WebSocket('ws://localhost:3001/ws?token=<jwt_token>');
```

## Message Format

All messages use JSON format:

```json
{
  "type": "message_type",
  "id": "unique_message_id", 
  "data": { /* message payload */ },
  "timestamp": "2025-09-07T12:00:00Z"
}
```

## Client → Server Messages

### Subscribe to Note Updates
```json
{
  "type": "subscribe_note",
  "id": "msg_001",
  "data": {
    "note_id": "uuid"
  }
}
```

### Unsubscribe from Note Updates
```json
{
  "type": "unsubscribe_note", 
  "id": "msg_002",
  "data": {
    "note_id": "uuid"
  }
}
```

### Send Note Operation (Operational Transform)
```json
{
  "type": "note_operation",
  "id": "msg_003", 
  "data": {
    "note_id": "uuid",
    "operation": {
      "type": "insert" | "delete" | "retain",
      "position": 42,
      "content": "text to insert",
      "length": 5, // for delete operations
      "attributes": {} // for formatting
    },
    "version": 15
  }
}
```

### Heartbeat (Keep-alive)
```json
{
  "type": "ping",
  "id": "msg_004",
  "data": {}
}
```

## Server → Client Messages

### Subscription Acknowledgment
```json
{
  "type": "subscribed",
  "id": "msg_001", // matches client message ID
  "data": {
    "note_id": "uuid",
    "current_version": 14,
    "collaborators": ["user_id_1", "user_id_2"]
  }
}
```

### Note Operation Broadcast
```json
{
  "type": "note_operation",
  "id": "server_msg_001",
  "data": {
    "note_id": "uuid", 
    "operation": {
      "type": "insert",
      "position": 42,
      "content": "inserted text",
      "author": "user_id_2"
    },
    "version": 16
  }
}
```

### Note Updated (Non-operational changes)
```json
{
  "type": "note_updated",
  "id": "server_msg_002",
  "data": {
    "note_id": "uuid",
    "title": "New Title",
    "version": 17,
    "updated_by": "user_id_1"
  }
}
```

### Folder Structure Changed
```json
{
  "type": "folder_updated", 
  "id": "server_msg_003",
  "data": {
    "folder_id": "uuid",
    "action": "created" | "updated" | "deleted" | "moved",
    "folder": { /* full folder object */ },
    "affected_notes": ["note_id_1", "note_id_2"]
  }
}
```

### User Presence Update
```json
{
  "type": "presence_update",
  "id": "server_msg_004", 
  "data": {
    "note_id": "uuid",
    "user_id": "uuid",
    "cursor_position": 125,
    "selection_start": 120,
    "selection_end": 130,
    "status": "active" | "idle" | "disconnected"
  }
}
```

### Error Response
```json
{
  "type": "error",
  "id": "msg_003", // matches failed client message ID
  "data": {
    "code": "VERSION_CONFLICT",
    "message": "Note was modified by another user",
    "current_version": 18
  }
}
```

### Heartbeat Response
```json
{
  "type": "pong",
  "id": "msg_004", // matches ping ID
  "data": {
    "server_time": "2025-09-07T12:00:00Z"
  }
}
```

## Operational Transform Operations

### Insert Operation
```json
{
  "type": "insert",
  "position": 42,
  "content": "Hello World",
  "attributes": {
    "bold": true,
    "italic": false
  }
}
```

### Delete Operation  
```json
{
  "type": "delete",
  "position": 42, 
  "length": 5
}
```

### Retain Operation (for formatting)
```json
{
  "type": "retain",
  "length": 10,
  "attributes": {
    "bold": true
  }
}
```

## Connection States

### Connection Flow
1. **Connecting**: WebSocket handshake in progress
2. **Connected**: Authenticated and ready to receive messages
3. **Subscribed**: Listening to specific note/folder updates
4. **Syncing**: Processing queued offline operations  
5. **Error**: Connection failed, attempting reconnection
6. **Disconnected**: Cleanly closed connection

### Reconnection Logic
- Exponential backoff: 1s, 2s, 4s, 8s, max 30s
- Resume subscriptions after reconnection
- Replay missed operations based on last known version
- Handle version conflicts during sync

## Error Codes

| Code | Description | Client Action |
|------|-------------|---------------|
| `UNAUTHORIZED` | Invalid or expired JWT | Redirect to login |
| `VERSION_CONFLICT` | Note modified by another user | Merge or reload note |
| `NOTE_NOT_FOUND` | Subscribed note was deleted | Update UI, unsubscribe |
| `RATE_LIMITED` | Too many operations per second | Queue operations, slow down |
| `INVALID_OPERATION` | Malformed operation data | Log error, skip operation |
| `SERVER_ERROR` | Internal server error | Show error message, retry |

## Performance Considerations

### Connection Limits
- Maximum 5 concurrent connections per user
- Maximum 50 note subscriptions per connection
- Operations rate limited to 100 ops/second per connection

### Message Batching
- Batch multiple operations in single WebSocket message
- Debounce rapid keystroke operations (50ms delay)
- Compress large operation payloads

### Conflict Resolution Priority
1. Operational Transform for concurrent edits
2. Last-write-wins for title/metadata changes
3. Server timestamp used for tie-breaking
4. User notification for lost changes

## Testing Scenarios

### Connection Tests
- Authenticate with valid/invalid tokens
- Handle connection drops and reconnection
- Test maximum connection limits

### Operation Tests  
- Apply operations in correct order
- Handle version conflicts gracefully
- Test operational transform correctness

### Real-time Tests
- Multiple users editing same note
- Rapid typing and operation queuing
- Network latency and message ordering