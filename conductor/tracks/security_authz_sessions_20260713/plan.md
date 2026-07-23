# Implementation Plan — Security: Channel Authorization & Server-Side Sessions

## Key Architecture Reference

- **Layering:** controllers (`src/auth.rs`, `src/channels.rs`, `src/attachments.rs`, `src/ws.rs`) parse and
  dispatch; business logic and transaction ownership live in `src/logic/`; SQL lives in `src/db/`. New code must
  follow this split: session/authorization *logic* goes in `src/logic/`, queries in `src/db/`.
- **Purity:** `ws_logic::decide()` (`src/logic/ws_logic.rs`) is a pure function returning `Vec<WsAction>` and must
  stay pure — membership checks are async/DB work, so they belong in the action-execution layer (`src/ws.rs` and
  the async functions in `ws_logic.rs`), not in `decide()`.
- **Auth today:** the signed cookie `session_id` currently stores a raw user UUID. It is read in three places:
  the `UserSession` extractor (`src/auth.rs`), the `/me` and `/logout` handlers (`src/auth.rs`), and the WS
  handshake (`src/ws.rs::ws_handler`). `AppState::validate_session` / `session_cache` are in `src/state.rs`.
- **Config:** `src/config.rs::Config` — values resolve env var → config file → default via `get_value`.
- **Tests:** backend tests live in `tests/` (separate files, tests first, helpers after). Integration tests must
  not open transactions themselves. E2E runs against 2 replicas (`docker-compose.e2e.yml`); cross-instance
  failures are real bugs.

---

## Phase 1: Session storage (DB + repository + logic) [checkpoint: 77ce09d]

### 1.1 Migration
- [x] Task: Create a new sqlx migration adding the `sessions` table per spec (id, user_id FK ON DELETE CASCADE,
  created_at, expires_at) and an index on `user_id`. Follow the naming style of existing files in `migrations/`.

### 1.2 Repository
- [x] Task: Write failing tests in `tests/db/sessions_tests.rs` (register in `tests/db/mod.rs`, follow the pattern
  of `tests/db/users_tests.rs` and the helpers in `tests/db/common.rs`) covering: create returns a session with
  the expected user_id and a future expiry; get-by-id returns `None` for unknown ids; get-by-id returns `None`
  for a row whose `expires_at` is in the past; delete removes the row.
- [x] Task: Implement `src/db/sessions.rs` (register in `src/db/mod.rs`) with functions taking
  `&mut Transaction<'_, Postgres>` like the other repositories: `create_session(tx, user_id, expires_at)`,
  `get_valid_session(tx, session_id) -> Option<Session>` (returns only non-expired rows; delete the row when it
  is found expired), `delete_session(tx, session_id)`.

### 1.3 Session logic
- [x] Task: Write failing tests (new file `tests/session_logic_tests.rs`) for a new `src/logic/sessions.rs`
  module: creating a session yields an id that validates back to the user; validation of a random id fails;
  validation after logout fails; the 30-day lifetime constant is applied (assert `expires_at ≈ now + 30d`).
- [x] Task: Implement `src/logic/sessions.rs` (register in `src/logic/mod.rs`): `start_session(pool, user_id) ->
  session_id`, `validate(state, session_id) -> Option<user_id>`, `end_session(state, session_id)`. This module
  owns the transactions. `validate` consults `AppState.session_cache` first (60 s TTL), then DB, then populates
  the cache. `end_session` deletes the DB row and evicts the cache entry.
- [x] Task: Re-key `AppState.session_cache` (`src/state.rs`) from `DashMap<Uuid, DateTime<Utc>>` (user_id) to
  `DashMap<Uuid, (Uuid, DateTime<Utc>)>` (session_id → user_id + cached_at). Move the cache read/write logic out
  of `AppState::validate_session` into `logic::sessions::validate`; `AppState::validate_session` and
  `invalidate_session` are removed or reduced to thin delegating wrappers (prefer removal — update callers).

### 1.4 Conductor — User Manual Verification 'Phase 1'
- [x] Task: Conductor — User Manual Verification 'Phase 1' (Protocol in workflow.md). `cargo test` green,
  `cargo clippy -- -D warnings` clean.

---

## Phase 2: Wire sessions into auth endpoints, extractor, and WS handshake [checkpoint: 97979b9]

### 2.1 Config flag
- [x] Task: Add `cookie_secure: bool` to `Config` (`src/config.rs`), default `false`, env `COOKIE_SECURE`,
  parsed like `max_attachment_size_bytes`. Extend `tests/config_tests.rs`.

### 2.2 Cookie issuance and attributes
- [x] Task: Update `set_session_cookie` in `src/auth.rs`: the value becomes the session id; set `http_only(true)`,
  `same_site(SameSite::Lax)`, `path("/")`, and `secure(config.cookie_secure)`. It will need the config — pass it
  in (the handlers can extract `State<CombinedState>`).
- [x] Task: `register` and `login` call `logic::sessions::start_session` after their existing DB work and put the
  returned session id in the cookie. `logout` parses the cookie as a session id and calls `end_session`.
  `me` validates via `logic::sessions::validate` and then loads the user.

### 2.3 Extractor and WS handshake
- [x] Task: Update the `UserSession` extractor (`src/auth.rs`) to parse the cookie as a session id and resolve it
  through `logic::sessions::validate`; reject with 401 otherwise.
- [x] Task: Update `ws_handler` (`src/ws.rs`) the same way: session id → validate → load user by the returned
  user_id. Keep the existing 401 behaviors for missing/invalid cookie.

### 2.4 Fix the test fleet
- [x] Task: The API/WS test helpers currently sign a raw user id into the cookie. Add a shared helper (e.g. in
  `tests/db/common.rs` or the existing API-test support code) that creates a user *and* a session row and returns
  the session-id cookie. Update `tests/api_tests.rs`, `tests/ws_tests.rs`, `tests/attachment_api_tests.rs`,
  `tests/channel_messages_api_tests.rs` to use it.
- [x] Task: Add integration tests for the new behavior: logout-then-replay yields 401 on `/api/auth/me`; an
  expired session (insert row with past `expires_at`) yields 401; Set-Cookie header on login contains `HttpOnly`
  and `SameSite=Lax`.

### 2.5 Conductor — User Manual Verification 'Phase 2'
- [x] Task: Conductor — User Manual Verification 'Phase 2' (Protocol in workflow.md). Manual check with two
  browser tabs: login, logout, verify the old tab's WS reconnect gets 401 (Network tab) and the app returns to
  the login screen.

---

## Phase 3: Channel authorization service and enforcement

### 3.1 Authorization service
- [x] Task: Write failing tests (`tests/authz_tests.rs`) for `src/logic/authz.rs`: member passes; non-member
  fails with a distinct error; unknown channel fails; `system.channels` is always allowed by the WS-facing
  variant.
- [x] Task: Implement `src/logic/authz.rs` with `ensure_channel_member(pool, user_id, channel_id) -> Result<(),
  AuthzError>` and a by-name variant for the WS path (resolve name → id via `state.channel_ids`, falling back to
  `db::channels::get_channel_by_name` like `ws_logic::resolve_channel_id` does). Reuse
  `db::channels::is_channel_member` for the query. `AuthzError` distinguishes `NotAMember` from `ChannelNotFound`
  and internal errors.

### 3.2 WebSocket enforcement
- [x] Task: In `src/ws.rs::handle_socket`, before executing `WsAction::Subscribe`, `WsAction::PublishToNats`, and
  `WsAction::PublishReadMarker`, check membership via the authz service. Maintain a per-connection
  `HashSet<String>` of already-verified channel names (alongside the existing `active_subscriptions`) so repeated
  sends don't re-query. On failure, send a STOMP `ERROR` frame via the existing `format_stomp_error` and skip the
  action. `system.channels` bypasses the check.
- [x] Task: Integration tests in `tests/ws_tests.rs`: non-member SUBSCRIBE gets ERROR and no MESSAGE frames;
  non-member SEND gets ERROR and the message is absent from the DB afterwards; member flows unchanged.

### 3.3 REST + attachment enforcement
- [x] Task: Switch `get_channel_messages` (`src/channels.rs`) to the shared authz function (403 on `NotAMember`).
- [x] Task: Attachment download authorization: extend the attachment lookup in `src/logic/attachments.rs` to also
  return `message_id` and `uploaded_by` (query in `src/db/attachments.rs`); in `get_attachment_for_download`,
  apply the spec rule (linked → member of the message's channel, which requires resolving the message's
  channel_id; unlinked → uploader only). Return `AppError::NotFound` for unauthorized access (don't reveal
  existence) or add a `Forbidden` variant — pick one and test it. Tests in `tests/attachment_api_tests.rs`.
- [x] Task: In `ws_logic::process_and_publish_message`, when resolving `attachment_ids`, filter to attachments
  whose `uploaded_by` equals the sender (adjust the fetch query); foreign ids are dropped silently. Unit-testable
  via the existing message-persistence integration tests.
- [x] Task: Private-channel guards: add `AND is_private = FALSE` to `search_unjoined_channels`
  (`src/db/channels.rs`); `join_channel` handler (`src/channels.rs`) returns 403 when the target channel is
  private. Tests for both.

### 3.4 Conductor — User Manual Verification 'Phase 3'
- [ ] Task: Conductor — User Manual Verification 'Phase 3' (Protocol in workflow.md).

---

## Phase 4: Full verification

- [ ] Task: Run the complete backend suite (`cargo test`), clippy with zero warnings, frontend build
  (`cd frontend; trunk build`), frontend tests.
- [ ] Task: Start the E2E stack (`docker compose -f docker-compose.e2e.yml --project-name sana-e2e up --build
  --wait`) and run the full Playwright suite (`cd e2e; npx playwright test --reporter=list`). The suite exercises
  login/register/messaging across 2 replicas — all tests must pass unmodified except where they assert on cookie
  internals.
- [ ] Task: Add one happy-path E2E assertion to `e2e/tests/auth.spec.ts`: after logout, navigating to the app
  lands on the login view (if not already covered).
- [ ] Task: Conductor — User Manual Verification 'Phase 4' (Protocol in workflow.md).
