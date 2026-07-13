# Implementation Plan — Correctness Quick Wins

## Key Architecture Reference

- **Router:** `src/router.rs::create_router` — layers are added here; `Config` is available via
  `CombinedState.config` before the router is built.
- **STOMP decision flow:** `src/logic/ws_logic.rs::decide()` is pure and returns `Vec<WsAction>`; `src/ws.rs::
  handle_socket` executes the actions. Receipt/error frames are formatted by `format_stomp_error` and inline
  `format!` calls in `ws.rs`.
- **Error bodies:** `src/auth.rs` defines the `ErrorResponse { error }` JSON shape — reuse it (move it to a shared
  location if that avoids duplication).
- **Frontend state:** `ChatState` + `ChatAction` in `frontend/src/logic.rs`, reducer arms in
  `frontend/src/state.rs`, STOMP parsing in `frontend/src/stomp.rs`, frame dispatch in
  `frontend/src/services/websocket.rs::handle_incoming_frame`, message rendering in
  `frontend/src/components/chat_window.rs` (see how `pending` is rendered — `failed` follows the same pattern).
- **Postgres unique violation:** SQLSTATE `23505`; see `is_foreign_key_violation` in `src/db/messages.rs` for the
  established way to inspect a `sqlx::Error` code.
- **Tests:** backend in `tests/` (tests first, helpers after, no transactions opened by the test itself);
  frontend unit tests in `frontend/tests/`; E2E in `e2e/tests/` with `data-testid` selectors only.

Each phase below is independent — implement in order but commit per phase.

---

## Phase 1: Body limit (B4)

- [ ] Task: Failing integration test in `tests/attachment_api_tests.rs`: upload a generated ~3 MB payload
  (allowed MIME) → expect 200 and a valid `AttachmentMeta`; upload one just over `max_attachment_size_bytes` →
  expect the JSON BadRequest from the handler's own size check.
- [ ] Task: In `src/router.rs`, add `axum::extract::DefaultBodyLimit::max(config.max_attachment_size_bytes as
  usize + 1024 * 1024)` as a router layer. Applying it router-wide is fine — JSON endpoints are still protected
  by their own semantics.
- [ ] Task: Conductor — User Manual Verification 'Phase 1' (Protocol in workflow.md).

---

## Phase 2: Receipt semantics + ERROR carries message_id (B8, server half of B9)

- [ ] Task: Failing unit tests in `tests/ws_logic_tests.rs` for the new `decide()` shape (below): a SEND with a
  `receipt` header no longer yields a standalone `SendReceipt` action; the receipt id travels with the publish
  action. SUBSCRIBE behavior unchanged.
- [ ] Task: Restructure the actions in `src/logic/ws_logic.rs`: `PublishToNats` and `PublishReadMarker` gain a
  `receipt_id: Option<String>` field; `decide()` attaches the header value there instead of emitting
  `SendReceipt` for SEND frames (`SendReceipt` remains for SUBSCRIBE).
- [ ] Task: In `src/ws.rs::handle_socket`, after a successful publish send `RECEIPT` (when a receipt id is
  present); on failure send `ERROR` including the `receipt-id` header *and* a `message_id:<id>` header when the
  message id is known. Extend `format_stomp_error` (or add a sibling formatter) for the extra header — keep it a
  pure, unit-tested function.
- [ ] Task: WS integration test in `tests/ws_tests.rs` for the success path (SEND with receipt → RECEIPT
  arrives). For the failure path prefer a unit test on the action-execution branch if the existing test harness
  cannot force a publish failure cheaply — do not build heavy NATS fault injection for this.
- [ ] Task: Conductor — User Manual Verification 'Phase 2' (Protocol in workflow.md).

---

## Phase 3: Frontend failed-message state (client half of B9)

- [ ] Task: Failing frontend unit tests (`frontend/tests/state_tests.rs`): new action `MarkMessageFailed
  { message_id }` sets `failed = true` on the matching pending message in whichever channel holds it; a message
  that already confirmed (has `seq`) is left untouched; unknown id is a no-op.
- [ ] Task: Add `#[serde(default)] pub failed: bool` to `ChatMessage` in `frontend/src/types.rs` (client-side
  only, like `pending`); add the `ChatAction` variant, `ChatState` method, and reducer arm following the
  existing pattern exactly.
- [ ] Task: Parse `message_id` out of ERROR frames: extend `StompFrame::Error` in `frontend/src/stomp.rs` to
  carry `message: String, message_id: Option<Uuid>` (parse the header like `receipt-id` is parsed). Unit tests
  in the frontend stomp tests.
- [ ] Task: In `handle_incoming_frame` (`frontend/src/services/websocket.rs`), on an ERROR with a message_id,
  emit a new callback that dispatches `MarkMessageFailed` (thread it through `use_chat_websocket` the same way
  `on_message` is threaded). Keep the existing console log for ERRORs without an id.
- [ ] Task: Render failed state in `frontend/src/components/chat_window.rs`: add a `failed` class on the
  message wrapper plus a small "failed to send" note; style it in `frontend/style.scss` (red accent). Add
  `data-testid="message-failed-indicator"` for future tests. A failed message must not also show as pending.
- [ ] Task: Conductor — User Manual Verification 'Phase 3' (Protocol in workflow.md). Manual check: stop NATS
  (`docker compose stop nats`) while the app runs, send a message, observe the failed indicator.

---

## Phase 4: Validation and 409s (B11)

- [ ] Task: Failing unit tests for a pure `validate_channel_name(&str) -> Result<String, String>` (returns the
  trimmed name or a human-readable reason) covering: empty/whitespace, >64 chars, control characters,
  `system.channels`, `system.` prefix, and a valid name passing through trimmed. Put the function in
  `src/logic/` (e.g. a small `channels` logic module) — validation is business logic, not controller code.
- [ ] Task: Use it in `create_channel` (`src/channels.rs`) → 400 with `ErrorResponse` JSON on violation. Note the
  handler currently returns bare `StatusCode` errors; switch its error type to `(StatusCode, Json<ErrorResponse>)`
  like `auth.rs`.
- [ ] Task: Map SQLSTATE `23505` to `409`: add a small helper `is_unique_violation(&sqlx::Error) -> bool` next to
  `is_foreign_key_violation` (`src/db/messages.rs` — or move both into a shared `db` util), and use it in
  `create_channel` and `register`. Integration tests: create the same channel name twice → second is 409;
  register the same username twice → second is 409.
- [ ] Task: Validate `limit` (1..=1000) in `get_channel_messages` → 400 otherwise. Extend
  `tests/channel_messages_api_tests.rs` with `limit=0`.
- [ ] Task: Conductor — User Manual Verification 'Phase 4' (Protocol in workflow.md).

---

## Phase 5: Dead code removal and full verification

- [ ] Task: Delete `frontend/src/communication.rs`. Verify with a workspace-wide grep that nothing references
  `communication` and that the file was never declared in `frontend/src/lib.rs`/`main.rs` (it isn't — it was
  dead), then build.
- [ ] Task: Full gate: `cargo test`, `cargo clippy -- -D warnings` (both crates), `cd frontend; trunk build`,
  full E2E suite on the 2-replica stack (`cd e2e; npx playwright test --reporter=list`).
- [ ] Task: Conductor — User Manual Verification 'Phase 5' (Protocol in workflow.md).
