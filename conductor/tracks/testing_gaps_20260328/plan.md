# Implementation Plan: Testing Gaps

## Phase 1: Backend Core Logic Tests [checkpoint: 3bf4a07]
- [x] Task: Extend `tests/ws_logic_tests.rs` with coverage for `handle_subscribe` and `process_and_publish_message`
- [x] Task: Create `tests/state_tests.rs` to verify concurrent channel registration
- [x] Task: Conductor - User Manual Verification 'Backend Core Logic Tests' (Protocol in workflow.md)

## Phase 2: Backend Infrastructure Tests
- [x] Task: Create `tests/archiver_tests.rs` with mock NATS/DB interactions
- [x] Task: Create `tests/nats_consumer_tests.rs` to verify broadcast relay
- [ ] Task: Conductor - User Manual Verification 'Backend Infrastructure Tests' (Protocol in workflow.md)

## Phase 3: Frontend Logic & Service Tests
- [ ] Task: Extend `frontend/tests/logic_tests.rs` with edge cases (Batch entries, out-of-order read markers)
- [ ] Task: Create `frontend/tests/websocket_service_tests.rs` with WebSocket mock
- [ ] Task: Conductor - User Manual Verification 'Frontend Logic & Service Tests' (Protocol in workflow.md)

## Phase 4: Frontend Hook Tests
- [ ] Task: Create `frontend/tests/hooks_tests.rs` for `use_channels`, `use_chat_scroll`, and `use_auth_check`
- [ ] Task: Conductor - User Manual Verification 'Frontend Hook Tests' (Protocol in workflow.md)

## Phase 5: Final Validation
- [ ] Task: Run all backend and frontend tests to ensure total system health
- [ ] Task: Conductor - User Manual Verification 'Final Validation' (Protocol in workflow.md)
