# Tasks: Fast Online Synchronized Note-Taking App

**Input**: Design documents from `/home/yasunori/fastest-note-app/specs/001-web-notion/`
**Prerequisites**: plan.md, research.md, data-model.md, contracts/, quickstart.md

## Execution Flow (main)
```
1. Load plan.md from feature directory
   → Tech stack: Rust (backend), Next.js (frontend), PostgreSQL, Redis
   → Structure: Web application (backend/ + frontend/ directories)
2. Load design documents:
   → data-model.md: User, Folder, Note entities
   → contracts/: 8 API endpoints + WebSocket spec
   → quickstart.md: 10 user journey scenarios
3. Generate tasks by category: Setup → Tests → Core → Integration → Polish
4. Apply TDD rules: All tests before implementation
5. Mark [P] for parallel execution (different files, no dependencies)
6. Number tasks T001-T040 sequentially
7. Generate dependency graph and parallel examples
8. Return: SUCCESS (40 tasks ready for execution)
```

## Format: `[ID] [P?] Description`
- **[P]**: Can run in parallel (different files, no dependencies)
- Include exact file paths in descriptions

## Path Conventions
- **Backend**: `backend/src/`, `backend/tests/`
- **Frontend**: `frontend/src/`, `frontend/tests/`
- Based on plan.md web application structure

## Phase 3.1: Setup

- [x] **T001** Create project structure: `backend/` and `frontend/` directories with Rust and Next.js scaffolding
- [x] **T002** Initialize Rust backend project with Axum, PostgreSQL, Redis dependencies in `backend/Cargo.toml`
- [x] **T003** Initialize Next.js frontend project with TypeScript, TanStack Query, IndexedDB in `frontend/package.json`
- [x] **T004** [P] Configure Rust linting and formatting tools: `backend/.clippy.toml`, `backend/rustfmt.toml`
- [x] **T005** [P] Configure frontend linting: `frontend/.eslintrc.json`, `frontend/prettier.config.js`
- [x] **T006** Create database migration files in `backend/migrations/001_initial_schema.sql` from data-model.md
- [x] **T007** [P] Setup Redis configuration and connection pool in `backend/src/redis.rs`
- [x] **T008** [P] Setup PostgreSQL connection pool with deadpool in `backend/src/database.rs`

## Phase 3.2: Tests First (TDD) ⚠️ MUST COMPLETE BEFORE 3.3

**CRITICAL: These tests MUST be written and MUST FAIL before ANY implementation**

### Authentication Contract Tests
- [x] **T009** [P] Contract test POST /auth/register in `backend/tests/contract/test_auth_register.rs`
- [x] **T010** [P] Contract test POST /auth/login in `backend/tests/contract/test_auth_login.rs` 
- [x] **T011** [P] Contract test POST /auth/refresh in `backend/tests/contract/test_auth_refresh.rs`

### Folder Contract Tests  
- [x] **T012** [P] Contract test GET /folders in `backend/tests/contract/test_folders_list.rs`
- [x] **T013** [P] Contract test POST /folders in `backend/tests/contract/test_folders_create.rs`
- [x] **T014** [P] Contract test PUT /folders/{id} in `backend/tests/contract/test_folders_update.rs`
- [x] **T015** [P] Contract test DELETE /folders/{id} in `backend/tests/contract/test_folders_delete.rs`

### Note Contract Tests
- [x] **T016** [P] Contract test GET /notes in `backend/tests/contract/test_notes_list.rs`
- [x] **T017** [P] Contract test POST /notes in `backend/tests/contract/test_notes_create.rs`
- [x] **T018** [P] Contract test PUT /notes/{id} in `backend/tests/contract/test_notes_update.rs`
- [x] **T019** [P] Contract test DELETE /notes/{id} in `backend/tests/contract/test_notes_delete.rs`
- [x] **T020** [P] Contract test POST /notes/{id}/move in `backend/tests/contract/test_notes_move.rs`

### WebSocket Contract Tests
- [x] **T021** [P] WebSocket connection and auth test in `backend/tests/contract/test_websocket_auth.rs`
- [x] **T022** [P] WebSocket note subscription test in `backend/tests/contract/test_websocket_subscribe.rs`
- [x] **T023** [P] WebSocket real-time operations test in `backend/tests/contract/test_websocket_operations.rs`

### Integration Tests from Quickstart Scenarios
- [x] **T024** [P] Integration test: User registration flow in `backend/tests/integration/test_user_registration.rs`
- [x] **T025** [P] Integration test: Note creation <200ms in `backend/tests/integration/test_note_performance.rs`
- [x] **T026** [P] Integration test: Folder hierarchy creation in `backend/tests/integration/test_folder_hierarchy.rs`
- [x] **T027** [P] Integration test: Note movement between folders in `backend/tests/integration/test_note_movement.rs`
- [x] **T028** [P] Integration test: Real-time synchronization in `backend/tests/integration/test_realtime_sync.rs`
- [x] **T029** [P] Integration test: Search functionality in `backend/tests/integration/test_search.rs`
- [x] **T030** [P] Integration test: Data size limits (1MB notes, 1000 items/folder) in `backend/tests/integration/test_data_limits.rs`
- [x] **T031** [P] Integration test: Conflict resolution in `backend/tests/integration/test_conflict_resolution.rs`

## Phase 3.3: Core Implementation (ONLY after tests are failing)

### Database Models
- [x] **T032** [P] User model with bcrypt password hashing in `backend/src/models/user.rs`
- [x] **T033** [P] Folder model with hierarchy path management in `backend/src/models/folder.rs`
- [x] **T034** [P] Note model with version tracking in `backend/src/models/note.rs`

### Service Layer
- [x] **T035** AuthService: registration, login, JWT generation in `backend/src/services/auth.rs`
- [x] **T036** FolderService: CRUD with hierarchy validation in `backend/src/services/folder.rs`
- [x] **T037** NoteService: CRUD with size/version validation in `backend/src/services/note.rs`
- [x] **T038** SyncService: WebSocket management and operational transforms in `backend/src/services/websocket.rs`

## Phase 3.4: API Implementation

### REST Endpoints
- [x] **T039** Authentication endpoints: `/auth/register`, `/auth/login`, `/auth/refresh` in `backend/src/handlers/auth.rs`
- [x] **T040** Folder endpoints: GET/POST/PUT/DELETE `/folders` in `backend/src/handlers/folder.rs`
- [x] **T041** Note endpoints: GET/POST/PUT/DELETE `/notes` + move in `backend/src/handlers/note.rs`
- [x] **T042** WebSocket handler for real-time sync in `backend/src/handlers/websocket.rs`

### Middleware & Infrastructure
- [x] **T043** JWT authentication middleware in `backend/src/middleware/auth.rs`
- [x] **T044** Request logging and error handling middleware in `backend/src/middleware/logging.rs`
- [x] **T045** CORS and security headers middleware in `backend/src/middleware/cors.rs`
- [x] **T046** Rate limiting middleware in `backend/src/middleware/rate_limit.rs`

## Phase 3.5: Frontend Implementation

### Core Components  
- [x] **T047** [P] Authentication forms: Login/Register in `frontend/src/components/Auth/`
- [x] **T048** [P] Note editor component with auto-save in `frontend/src/components/Editor/NoteEditor.tsx`
- [x] **T049** [P] Folder tree navigation component in `frontend/src/components/Navigation/FolderTree.tsx`
- [x] **T050** [P] Note list with virtual scrolling in `frontend/src/components/Notes/NoteList.tsx`

### State Management & API Integration
- [x] **T051** API client with TanStack Query integration in `frontend/src/services/api.ts`
- [x] **T052** Authentication state management in `frontend/src/stores/authStore.ts`
- [x] **T053** Notes and folders state with optimistic updates in `frontend/src/stores/contentStore.ts`
- [x] **T054** WebSocket service for real-time updates in `frontend/src/services/websocket.ts`

### Offline Support
- [x] **T055** IndexedDB service for offline storage in `frontend/src/services/offline.ts`
- [x] **T056** Sync queue for offline operations in `frontend/src/services/syncQueue.ts`
- [x] **T057** Conflict resolution UI components in `frontend/src/components/Sync/ConflictResolver.tsx`

## Phase 3.6: Performance & Polish

### Performance Optimization
- [x] **T058** [P] Database query optimization and indexing in `backend/src/database/optimizations.sql`
- [x] **T059** [P] Redis caching layer for frequently accessed data in `backend/src/cache/redis_cache.rs`
- [x] **T060** [P] Frontend bundle optimization and code splitting in `frontend/next.config.js`

### Testing & Validation  
- [x] **T061** [P] Unit tests for validation logic in `backend/tests/unit/test_validation.rs`
- [x] **T062** [P] Frontend unit tests with Jest in `frontend/tests/unit/`
- [x] **T063** [P] End-to-end tests with Playwright in `frontend/tests/e2e/`
- [x] **T064** Performance benchmark tests (<200ms API) in `backend/tests/performance/`
- [x] **T065** Execute quickstart validation scenarios in `specs/001-web-notion/quickstart.md`

### Documentation & Cleanup
- [x] **T066** [P] API documentation generation from OpenAPI spec
- [x] **T067** [P] Frontend component documentation with Storybook
- [x] **T068** Code cleanup and remove TODO comments
- [x] **T069** Final security audit and dependency updates

## Dependencies

### Critical Path (Must Complete In Order)
1. **Setup** (T001-T008) → **Contract Tests** (T009-T031) → **Implementation** (T032+)
2. **Models** (T032-T034) before **Services** (T035-T038) 
3. **Services** before **API endpoints** (T039-T042)
4. **Backend API** before **Frontend API client** (T051)
5. **Core components** (T047-T050) before **Advanced features** (T055-T057)

### Blocking Dependencies  
- T032-T034 (models) block T035-T038 (services)
- T035 (AuthService) blocks T043 (auth middleware)  
- T039-T042 (API endpoints) block T051 (frontend API client)
- T051 (API client) blocks T052-T054 (frontend state)
- All tests (T009-T031) must FAIL before implementation (T032+)

## Parallel Execution Examples

### Phase 3.2: All Contract Tests (Can Run Simultaneously)
```bash
# Launch T009-T031 together (23 parallel test tasks):
Task: "Contract test POST /auth/register in backend/tests/contract/test_auth_register.rs"  
Task: "Contract test GET /folders in backend/tests/contract/test_folders_list.rs"
Task: "Integration test: Note creation <200ms in backend/tests/integration/test_note_performance.rs"
# ... (all contract and integration tests)
```

### Phase 3.3: Model Creation (Independent Files)
```bash
# Launch T032-T034 together:
Task: "User model with bcrypt password hashing in backend/src/models/user.rs"
Task: "Folder model with hierarchy path management in backend/src/models/folder.rs" 
Task: "Note model with version tracking in backend/src/models/note.rs"
```

### Phase 3.5: Frontend Components (Independent Components)
```bash
# Launch T047-T050 together:
Task: "Authentication forms: Login/Register in frontend/src/components/Auth/"
Task: "Note editor component with auto-save in frontend/src/components/Editor/NoteEditor.tsx"
Task: "Folder tree navigation component in frontend/src/components/Navigation/FolderTree.tsx"
Task: "Note list with virtual scrolling in frontend/src/components/Notes/NoteList.tsx"
```

## Notes
- **[P] tasks** = different files, no dependencies - can run in parallel
- **Verify tests fail** before implementing (TDD requirement)
- **Commit after each task** for clean git history
- **Performance targets**: <200ms API responses, <100ms UI interactions
- **Data limits**: 1MB max note size, 1000 items per folder, 10-level folder depth

## Validation Checklist
*GATE: Must be completed before marking feature complete*

- [x] All 8 API endpoints have contract tests (T009-T020)
- [x] All 3 entities (User, Folder, Note) have model tasks (T032-T034)
- [x] All 10 quickstart scenarios have integration tests (T024-T031)
- [x] WebSocket real-time functionality tested (T021-T023)
- [x] Tests come before implementation (Phase 3.2 → 3.3)
- [x] Parallel tasks are truly independent (different files)
- [x] Each task specifies exact file path
- [x] Performance requirements validated (<200ms)
- [x] Offline functionality implemented (T055-T057)
- [x] Security measures included (auth, rate limiting, validation)

**Total Tasks**: 69 tasks across 6 phases
**Estimated Parallel Groups**: 8 groups can run simultaneously  
**Critical Path**: ~12-15 sequential steps with optimal parallelization