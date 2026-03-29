# Implementation Plan: Error Handling & Resilience

## Phase 1: Backend Resilience [checkpoint: 3b9df6c]
- [x] Task: Add `anyhow` dependency, fix `main.rs` startup error handling
- [x] Task: Add channel membership check in `get_channel_messages()`
- [x] Task: Implement session cache with TTL in `auth.rs`
- [x] Task: Add NATS publish error logging/handling
- [x] Task: Fix `logic/nats.rs` stream lookup error handling
- [x] Task: Add global memory limit to `MessageStore`
- [x] Task: Conductor - User Manual Verification 'Backend Resilience' (Protocol in workflow.md)

## Phase 2: Frontend Resilience
- [x] Task: Cap WebSocket reconnection retries with exponential backoff
- [x] Task: Add timeouts to frontend HTTP requests
- [x] Task: Conductor - User Manual Verification 'Frontend Resilience' (Protocol in workflow.md)

## Phase 3: Final System-Wide Validation
- [ ] Task: Run full test suite (`cargo test` in backend and frontend)
- [ ] Task: Final manual end-to-end verification (stop NATS/DB, verify resilience)
- [ ] Task: Conductor - User Manual Verification 'Final System-Wide Validation' (Protocol in workflow.md)
