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
