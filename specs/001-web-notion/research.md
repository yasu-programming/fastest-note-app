# Phase 0: Research & Technology Decisions

## Backend Language & Framework

**Decision**: Rust with Axum web framework  
**Rationale**: 
- Rust provides memory safety with zero-cost abstractions
- Axum offers excellent performance (sub-millisecond response times achievable)
- Strong ecosystem for async programming with tokio
- Better performance than Next.js API routes for intensive operations

**Alternatives Considered**:
- Go with Gin/Fiber: Good performance but slightly slower than Rust
- Node.js with Fastify: Familiar but single-threaded limitations
- Actix-web vs Axum: Chose Axum for better ergonomics and Tower ecosystem

## Frontend Framework

**Decision**: Next.js 14 with App Router  
**Rationale**:
- React Server Components reduce client-side JavaScript
- Built-in optimizations for performance (Image, Font optimization)
- Excellent TypeScript integration
- Strong ecosystem for UI components

**Alternatives Considered**:
- SvelteKit: Faster but smaller ecosystem
- Solid.js: Great performance but less mature tooling
- Vanilla React with Vite: More setup, less optimization out-of-box

## Database Architecture

**Decision**: Dual database approach
- **Primary**: PostgreSQL with optimized schema and indexes
- **Cache**: Redis for session data, frequently accessed notes
- **Client**: IndexedDB for offline storage

**Rationale**:
- PostgreSQL provides ACID compliance for data integrity
- JSON columns for flexible note content storage
- Redis enables sub-10ms cache lookups
- IndexedDB supports offline editing with 1MB+ storage per note

**Alternatives Considered**:
- ClickHouse: Excellent for analytics but overkill for this use case
- MongoDB: Good for document storage but less performant for hierarchical queries
- SQLite: Great for single-user but doesn't scale to 10k+ users

## Real-time Synchronization

**Decision**: WebSocket connections with operational transform conflict resolution  
**Rationale**:
- WebSockets provide bidirectional real-time communication
- Operational transforms handle concurrent editing elegantly
- Can fallback to HTTP polling if WebSocket fails

**Alternatives Considered**:
- Server-Sent Events: Unidirectional, requires HTTP for client→server
- Long polling: Higher latency and resource usage
- Last-write-wins: Simpler but loses user data

## Offline Support Strategy

**Decision**: Local-first architecture with sync queue  
**Rationale**:
- IndexedDB stores full note content locally
- Sync queue handles operations during offline periods
- Conflict resolution on reconnection with user notifications

**Implementation Details**:
- Store operations in indexed queue during offline
- Batch sync operations on reconnection
- Use optimistic updates for immediate UI feedback

## Performance Optimizations

**Backend Optimizations**:
- Connection pooling with deadpool-postgres
- Database indexes on folder hierarchies and search content
- Rust zero-copy serialization with serde
- Compression for large note content

**Frontend Optimizations**:
- Virtual scrolling for large folder contents
- React.memo for note list items
- TanStack Query for intelligent caching and background updates
- Code splitting for faster initial loads

## Authentication & Security

**Decision**: JWT-based authentication with refresh tokens  
**Rationale**:
- Stateless authentication scales well
- Refresh tokens provide security without constant re-auth
- Easy to implement across different devices

**Security Measures**:
- bcrypt for password hashing
- Rate limiting on authentication endpoints  
- HTTPS-only cookies for refresh tokens
- Input validation and sanitization

## Testing Strategy

**Backend Testing**:
- Unit tests with cargo test
- Integration tests with real PostgreSQL (testcontainers)
- Contract tests for API endpoints
- Load testing with criterion.rs

**Frontend Testing**:
- Unit tests with Jest and React Testing Library
- Integration tests with Playwright
- Visual regression tests for UI components
- End-to-end user journey tests

## Deployment Architecture

**Decision**: Containerized deployment with Docker  
**Rationale**:
- Consistent environments across development/production
- Easy scaling with container orchestration
- Simplified dependency management

**Infrastructure**:
- Backend: Rust application in Alpine Linux container
- Frontend: Static files served by CDN/nginx
- Database: PostgreSQL container with persistent volumes
- Cache: Redis container for session management