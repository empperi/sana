# Specification: Testing Gaps

## Overview
Add tests for critical untested code paths identified in the Sana codebase. This includes complex logic in both the backend (message routing, archiving, concurrent state) and the frontend (WebSocket service, hooks).

## Functional Requirements
- **Backend Test Coverage**:
    - `src/logic/ws_logic.rs`: Test subscription handling, message routing, and batching.
    - `src/logic/archiver.rs`: Test JetStream consumption, persistence logic, and foreign key violation handling.
    - `src/logic/nats.rs`: Test broadcast relay and system message processing.
    - `src/state.rs`: Test concurrent access patterns and state initialization.
- **Frontend Test Coverage**:
    - `frontend/src/services/websocket.rs`: Test reconnection logic, receipt tracking, and message buffering.
    - `frontend/src/hooks/*`: Test component lifecycle hooks and state updates.
    - `frontend/src/logic.rs`: Extend tests for edge cases in state mutations.

## Non-Functional Requirements
- **Test Quality**: Adhere to `AGENTS.md` rules: unit tests in separate files, tests before fixtures, no testing of private functions.
- **Maintainability**: Use mocks where appropriate (e.g., NATS, WebSocket) to keep tests fast and isolated.

## Acceptance Criteria
- All newly added tests pass.
- Zero compilation warnings in test code.
- Coverage for identified critical paths is established.
- Integration tests (if any) do not open database transactions.
