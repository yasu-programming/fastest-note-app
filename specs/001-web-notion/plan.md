# Implementation Plan: Fast Online Synchronized Note-Taking App

**Branch**: `001-web-notion` | **Date**: 2025-09-07 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/home/yasunori/fastest-note-app/specs/001-web-notion/spec.md`

## Execution Flow (/plan command scope)
```
1. Load feature spec from Input path
   → If not found: ERROR "No feature spec at {path}"
2. Fill Technical Context (scan for NEEDS CLARIFICATION)
   → Detect Project Type from context (web=frontend+backend, mobile=app+api)
   → Set Structure Decision based on project type
3. Evaluate Constitution Check section below
   → If violations exist: Document in Complexity Tracking
   → If no justification possible: ERROR "Simplify approach first"
   → Update Progress Tracking: Initial Constitution Check
4. Execute Phase 0 → research.md
   → If NEEDS CLARIFICATION remain: ERROR "Resolve unknowns"
5. Execute Phase 1 → contracts, data-model.md, quickstart.md, agent-specific template file (e.g., `CLAUDE.md` for Claude Code, `.github/copilot-instructions.md` for GitHub Copilot, or `GEMINI.md` for Gemini CLI).
6. Re-evaluate Constitution Check section
   → If new violations: Refactor design, return to Phase 1
   → Update Progress Tracking: Post-Design Constitution Check
7. Plan Phase 2 → Describe task generation approach (DO NOT create tasks.md)
8. STOP - Ready for /tasks command
```

**IMPORTANT**: The /plan command STOPS at step 7. Phases 2-4 are executed by other commands:
- Phase 2: /tasks command creates tasks.md
- Phase 3-4: Implementation execution (manual or via tools)

## Summary
Fast note-taking web application with hierarchical folder organization and real-time cross-device synchronization. Performance target: sub-200ms response times. Uses Rust backend for maximum speed, Next.js frontend for optimal user experience, and high-performance database for instant data access.

## Technical Context
**Language/Version**: Rust 1.75+ (backend), TypeScript/Next.js 14+ (frontend)  
**Primary Dependencies**: Axum/Actix-web (Rust web framework), Next.js, TanStack Query (data fetching)  
**Storage**: Redis (caching), PostgreSQL or ClickHouse (primary data), IndexedDB (offline storage)  
**Testing**: cargo test (Rust), Jest/Playwright (frontend), integration tests with real databases  
**Target Platform**: Web browsers (Chrome/Firefox/Safari), Linux/Docker deployment
**Project Type**: web - determines frontend/backend structure  
**Performance Goals**: <200ms API response time, <100ms UI interactions, 1000+ concurrent users  
**Constraints**: <200ms p95 response time, offline-capable, real-time sync, 1MB max note size  
**Scale/Scope**: 10k+ users, 1M+ notes, hierarchical organization up to 10 levels deep

## Constitution Check
*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

**Simplicity**:
- Projects: 2 (backend API, frontend web app) ✅
- Using framework directly? Yes (Axum for backend, Next.js for frontend) ✅ 
- Single data model? Yes (shared schema between frontend/backend) ✅
- Avoiding patterns? Yes (direct DB access, no Repository pattern) ✅

**Architecture**:
- EVERY feature as library? ✅ (note-service, folder-service, auth-service libs)
- Libraries listed: auth-service (JWT/sessions), note-service (CRUD), folder-service (hierarchy), sync-service (real-time)
- CLI per library: ✅ (--help/--version/--json for each service)
- Library docs: ✅ (llms.txt format planned for each service)

**Testing (NON-NEGOTIABLE)**:
- RED-GREEN-Refactor cycle enforced? ✅ (contract tests first, then implementation)
- Git commits show tests before implementation? ✅ (TDD workflow enforced)
- Order: Contract→Integration→E2E→Unit strictly followed? ✅
- Real dependencies used? ✅ (PostgreSQL, Redis - no mocks)
- Integration tests for: ✅ (API contracts, WebSocket sync, database schemas)
- FORBIDDEN: Implementation before test, skipping RED phase ✅

**Observability**:
- Structured logging included? ✅ (JSON logs with tracing IDs)
- Frontend logs → backend? ✅ (unified logging stream planned)
- Error context sufficient? ✅ (request IDs, user context, error codes)

**Versioning**:
- Version number assigned? ✅ (1.0.0 - MAJOR.MINOR.BUILD)
- BUILD increments on every change? ✅ (automated via CI)
- Breaking changes handled? ✅ (API versioning, migration scripts planned)

## Project Structure

### Documentation (this feature)
```
specs/[###-feature]/
├── plan.md              # This file (/plan command output)
├── research.md          # Phase 0 output (/plan command)
├── data-model.md        # Phase 1 output (/plan command)
├── quickstart.md        # Phase 1 output (/plan command)
├── contracts/           # Phase 1 output (/plan command)
└── tasks.md             # Phase 2 output (/tasks command - NOT created by /plan)
```

### Source Code (repository root)
```
# Option 1: Single project (DEFAULT)
src/
├── models/
├── services/
├── cli/
└── lib/

tests/
├── contract/
├── integration/
└── unit/

# Option 2: Web application (when "frontend" + "backend" detected)
backend/
├── src/
│   ├── models/
│   ├── services/
│   └── api/
└── tests/

frontend/
├── src/
│   ├── components/
│   ├── pages/
│   └── services/
└── tests/

# Option 3: Mobile + API (when "iOS/Android" detected)
api/
└── [same as backend above]

ios/ or android/
└── [platform-specific structure]
```

**Structure Decision**: Option 2 (Web application) - separate backend and frontend projects

## Phase 0: Outline & Research
1. **Extract unknowns from Technical Context** above:
   - For each NEEDS CLARIFICATION → research task
   - For each dependency → best practices task
   - For each integration → patterns task

2. **Generate and dispatch research agents**:
   ```
   For each unknown in Technical Context:
     Task: "Research {unknown} for {feature context}"
   For each technology choice:
     Task: "Find best practices for {tech} in {domain}"
   ```

3. **Consolidate findings** in `research.md` using format:
   - Decision: [what was chosen]
   - Rationale: [why chosen]
   - Alternatives considered: [what else evaluated]

**Output**: research.md with all NEEDS CLARIFICATION resolved

## Phase 1: Design & Contracts
*Prerequisites: research.md complete*

1. **Extract entities from feature spec** → `data-model.md`:
   - Entity name, fields, relationships
   - Validation rules from requirements
   - State transitions if applicable

2. **Generate API contracts** from functional requirements:
   - For each user action → endpoint
   - Use standard REST/GraphQL patterns
   - Output OpenAPI/GraphQL schema to `/contracts/`

3. **Generate contract tests** from contracts:
   - One test file per endpoint
   - Assert request/response schemas
   - Tests must fail (no implementation yet)

4. **Extract test scenarios** from user stories:
   - Each story → integration test scenario
   - Quickstart test = story validation steps

5. **Update agent file incrementally** (O(1) operation):
   - Run `/scripts/update-agent-context.sh [claude|gemini|copilot]` for your AI assistant
   - If exists: Add only NEW tech from current plan
   - Preserve manual additions between markers
   - Update recent changes (keep last 3)
   - Keep under 150 lines for token efficiency
   - Output to repository root

**Output**: data-model.md, /contracts/*, failing tests, quickstart.md, agent-specific file

## Phase 2: Task Planning Approach
*This section describes what the /tasks command will do - DO NOT execute during /plan*

**Task Generation Strategy**:
- Load `/templates/tasks-template.md` as base
- Generate tasks from Phase 1 design docs (contracts, data model, quickstart)
- Each API endpoint → contract test task [P]
- Each database entity → model creation task [P]
- WebSocket operations → real-time sync test tasks
- Each user story from quickstart → integration test task
- Implementation tasks to make all tests pass

**Ordering Strategy**:
- TDD order: Contract tests → Integration tests → Implementation
- Dependency order: Database setup → Models → Services → API → Frontend
- Backend services can be developed in parallel [P]
- Frontend components developed after API contracts stable
- Mark [P] for parallel execution (independent services)

**Expected Task Categories**:
1. Database setup and migrations (PostgreSQL, Redis)
2. Authentication service with contract tests
3. Note service with CRUD operations
4. Folder service with hierarchy management  
5. Real-time sync service with WebSocket
6. Frontend components and API integration
7. Offline storage and sync queue
8. Performance optimization and caching

**Estimated Output**: 35-40 numbered, ordered tasks in tasks.md

**IMPORTANT**: This phase is executed by the /tasks command, NOT by /plan

## Phase 3+: Future Implementation
*These phases are beyond the scope of the /plan command*

**Phase 3**: Task execution (/tasks command creates tasks.md)  
**Phase 4**: Implementation (execute tasks.md following constitutional principles)  
**Phase 5**: Validation (run tests, execute quickstart.md, performance validation)

## Complexity Tracking
*Fill ONLY if Constitution Check has violations that must be justified*

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| [e.g., 4th project] | [current need] | [why 3 projects insufficient] |
| [e.g., Repository pattern] | [specific problem] | [why direct DB access insufficient] |


## Progress Tracking
*This checklist is updated during execution flow*

**Phase Status**:
- [x] Phase 0: Research complete (/plan command) - research.md generated
- [x] Phase 1: Design complete (/plan command) - data-model.md, contracts/, quickstart.md generated
- [x] Phase 2: Task planning complete (/plan command - describe approach only)
- [ ] Phase 3: Tasks generated (/tasks command)
- [ ] Phase 4: Implementation complete
- [ ] Phase 5: Validation passed

**Gate Status**:
- [x] Initial Constitution Check: PASS (2 projects, library architecture, TDD enforced)
- [x] Post-Design Constitution Check: PASS (no complexity violations introduced)
- [x] All NEEDS CLARIFICATION resolved (research.md addresses all technical decisions)
- [x] Complexity deviations documented (none - within constitutional guidelines)

---
*Based on Constitution v2.1.1 - See `/memory/constitution.md`*