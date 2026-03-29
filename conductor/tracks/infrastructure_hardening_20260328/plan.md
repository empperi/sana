# Implementation Plan: Infrastructure Hardening

## Phase 1: Docker & Nginx Optimization
- [x] Task: Add pgdata volume to `docker-compose.yml` for persistence
- [x] Task: Add gzip compression to `nginx.conf`
- [x] Task: Add cache-control headers for static assets in `nginx.conf`
- [x] Task: Add resource limits (CPU/Memory) to all services in `docker-compose.yml`
- [x] Task: Move `COOKIE_KEY` to `.env` reference in `docker-compose.yml`
- [x] Task: Conductor - User Manual Verification 'Docker & Nginx' (Protocol in workflow.md)

## Phase 2: Application Health & Config [checkpoint: 4e06451]
- [x] Task: Add `/health` endpoint to backend in `src/router.rs`
- [x] Task: Add health check to `app` service in `docker-compose.yml`
- [x] Task: Make CORS origin configurable in `src/config.rs` and `src/router.rs`
- [x] Task: Conductor - User Manual Verification 'Health & Config' (Protocol in workflow.md)

## Phase 3: Final System-Wide Validation
- [x] Task: Run full stack with `docker-compose up --build -d` and verify health status
- [x] Task: Verify static asset headers and gzip using `curl -I`
- [x] Task: Verify data persistence by restarting the stack
- [x] Task: Conductor - User Manual Verification 'Final System-Wide Validation' (Protocol in workflow.md)
