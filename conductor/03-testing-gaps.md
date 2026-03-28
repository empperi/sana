# Phase 3: Testing Gaps

## Objective
Add tests for the critical untested code paths. Currently, the most complex and failure-prone
parts of both backend and frontend have no test coverage.

## Current Coverage

### What IS tested (good)
- STOMP parsing: `tests/stomp_tests.rs`
- NATS utilities: `tests/nats_util_tests.rs`
- Configuration: `tests/config_tests.rs`
- DB repositories: `tests/users_tests.rs`, `tests/channels_tests.rs`, `tests/messages_tests.rs`,
  `tests/messages_history_tests.rs`
- REST API endpoints: `tests/api_tests.rs`, `tests/channel_messages_api_tests.rs`,
  `tests/message_persistence_tests.rs`
- WebSocket basics: `tests/ws_tests.rs`
- Frontend state logic: `frontend/tests/logic_tests.rs` (13 tests)
- Frontend STOMP: `frontend/tests/stomp_tests.rs`

### What is NOT tested (gaps)

| File | Lines | Complexity | Risk |
|------|-------|------------|------|
| `src/logic/ws_logic.rs` | 292 | Very high — subscription, message routing, channel resolution | Messages lost or duplicated |
| `src/logic/archiver.rs` | 300 | High — JetStream consumption, persistence, FK handling | Data loss on failure |
| `src/logic/nats.rs` | 90 | Medium — NATS consumer, broadcast relay | Silent message drops |
| `src/state.rs` | 50 | Medium — concurrent access patterns | Race conditions |
| `frontend/src/services/websocket.rs` | 339 | Very high — reconnection, receipt tracking, buffer mgmt | Connection hangs |
| `frontend/src/hooks/*` | ~350 | Medium — lifecycle, scroll, subscriptions | UI state bugs |
| `frontend/src/components/*` | ~740 | Low — mostly rendering | Visual regressions |

## Proposed Test Files

### Backend

**`tests/ws_logic_tests.rs`** — Highest priority

Test the pure functions extracted in Phase 2:
- `fetch_db_history()` — returns correct entries, respects `last_seen_seq`
- `merge_and_deduplicate()` — no duplicates, correct ordering
- `resolve_channel_id()` — valid destination, missing channel, invalid prefix
- `build_channel_entry()` — correct field population for chat and system messages
- `send_in_batches()` — batches of correct size, handles empty input
- `decide()` — correct action for each STOMP command (SUBSCRIBE, SEND, DISCONNECT)

Since Phase 2 extracts pure functions, these become straightforward unit tests with no DB/NATS
dependency.

**`tests/archiver_tests.rs`** — High priority

- Consumer creation with correct deliver policy (when `max_seq` exists vs. first-time)
- Message processing: valid message persisted, acknowledged
- FK violation handling: orphan message nacked, not persisted
- Batch processing: multiple messages in sequence
- Recovery: consumer restarts from correct sequence number

These tests need a mock or embedded NATS. Consider `async-nats` test utilities or a thin
mock layer.

**`tests/nats_consumer_tests.rs`** — Medium priority

- Broadcast relay: message received on NATS subject arrives on correct broadcast channel
- System message handling: channel creation events update state
- Malformed message handling: invalid JSON is logged and skipped
- Channel not found: message for unknown channel is handled gracefully

**`tests/state_tests.rs`** — Medium priority

- Concurrent channel registration: multiple tasks adding channels simultaneously
- `load_channels_from_db()`: channels from DB appear in state after load
- Channel lookup: correct UUID returned for known channel name

After Phase 2 (DashMap migration), these tests verify the concurrent access patterns are safe.

### Frontend

**`frontend/tests/websocket_service_tests.rs`** — High priority

Testing the WebSocket service requires mocking the WebSocket connection. Focus on the
state machine transitions:
- Connection established: status changes to Connected
- Connection lost: status changes to Disconnected, reconnect initiated
- Message received: callback invoked with correct parsed data
- Send buffer: messages queued while disconnected are sent on reconnect
- Receipt tracking: subscription receipts tracked and cleared
- Max reconnection: verify backoff ceiling behavior

**`frontend/tests/hooks_tests.rs`** — Medium priority

After Phase 1 (context refactor), hooks become easier to test in isolation:
- `use_channels`: dispatches SetChannels after auth completes
- `use_chat_scroll`: auto-scroll when at bottom, preserve position when scrolled up
- `use_auth_check`: redirects on 401, stays on 200

**`frontend/tests/logic_tests.rs`** — Extend existing

Add tests for edge cases not currently covered:
- `prepend_historical_messages()` with overlapping sequences (deduplication)
- `handle_system_message()` for each system message type
- `handle_message()` with Batch entries (recursive processing)
- State with empty channel list (edge case)
- Read marker updates with out-of-order messages

## Implementation Steps

1. Write `ws_logic_tests.rs` — depends on Phase 2 function extraction
2. Write `archiver_tests.rs` — requires mock NATS setup helper
3. Write `nats_consumer_tests.rs` — can share mock NATS helper
4. Write `state_tests.rs` — straightforward after DashMap migration
5. Extend `frontend/tests/logic_tests.rs` — independent of other phases
6. Write `frontend/tests/websocket_service_tests.rs` — requires WebSocket mock
7. Write `frontend/tests/hooks_tests.rs` — depends on Phase 1 context refactor

## Dependencies

- **Phase 2** must complete before `ws_logic_tests.rs` (needs extracted pure functions)
- **Phase 1** must complete before `hooks_tests.rs` (needs context-based state)
- `logic_tests.rs` extensions and `archiver_tests.rs` can start independently

## Test Quality Rules (from AGENTS.md)

- Tests go in separate files under `tests/` or `frontend/tests/`
- Tests come first in the file, fixtures and helpers after
- Do not test private functions — test via public API
- Unit tests preferred over integration tests
- Integration tests must not open DB transactions
