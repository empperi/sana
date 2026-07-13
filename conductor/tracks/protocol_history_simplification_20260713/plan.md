# Implementation Plan — Protocol Documentation & History Simplification

## Key Architecture Reference

- **Subscribe flow today:** `src/logic/ws_logic.rs::handle_subscribe` — (1) metadata entry, (2) subscribe to the
  broadcast first to buffer live messages, (3) DB history via `db::messages::get_messages(..., order_asc=true)`,
  (4) mem history from `MessageStore` filtered by `last_db_seq`, (5) `merge_and_deduplicate`, (6) batches of 20,
  (7) `spawn_forwarding_task`. Steps 4–5 are what this track deletes; steps 1–3, 6–7 survive.
- **MessageStore:** `src/messages.rs` (struct + eviction), populated in `src/logic/nats.rs::handle_chat_message`,
  field on `AppState` (`src/state.rs`).
- **REST history:** `src/channels.rs::get_channel_messages` → `db::messages::get_messages` (timestamp-ordered,
  `before` param). Frontend consumer: `fetch_historical_messages` + `on_load_history` in `frontend/src/main.rs`,
  scroll trigger in `frontend/src/hooks/use_chat_scroll.rs`, prepend logic in
  `frontend/src/logic.rs::prepend_historical_messages`.
- **JetStream replay primitives:** the stream is `SANA`, subjects `topic.<hex>` (`nats_util::encode`). An
  ordered pull consumer accepts `filter_subject` and `DeliverPolicy::ByStartSequence`. The end of the replay
  window is known up front from `stream.info().state.last_sequence` (capture it *before* draining; anything
  newer is covered by the buffered live broadcast + client-side dedup by id).
- **Serialization detail:** `ChannelEntry` serializes with `{"type": ..., "data": ...}` tags — the REST response
  change is exactly this envelope; frontend `types.rs::ChannelEntry` already matches.
- **Tests:** `tests/ws_logic_tests.rs`, `tests/channel_messages_api_tests.rs`, `tests/message_persistence_tests.rs`
  cover the touched areas; frontend tests in `frontend/tests/`.

---

## Phase 1: Wire protocol documentation

- [ ] Task: Write `docs/wire-protocol.md` per the spec's content list. Source of truth is the code — read
  `src/stomp.rs` (inbound parsing), `src/logic/ws_logic.rs` (outbound frame formatting, subscribe sequence),
  `frontend/src/stomp.rs` (client side), and `src/messages.rs::ChannelEntry` (JSON schema; include one JSON
  example per variant). Include a sequence diagram (Mermaid) of connect → subscribe → history batches → live
  messages → reconnect-with-last_seen_seq.
- [ ] Task: Cross-check every documented header against both parsers; where doc and code disagree, the code
  wins — fix the doc, and note genuine oddities in the divergences section rather than changing behavior.
- [ ] Task: Conductor — User Manual Verification 'Phase 1' (Protocol in workflow.md) — a read-through review.

---

## Phase 2: Seq-ordered, ChannelEntry-based REST history (backend)

### 2.1 Repository
- [ ] Task: Migration: `CREATE INDEX IF NOT EXISTS idx_messages_channel_seq ON messages (channel_id, seq DESC);`
- [ ] Task: TDD in `tests/db/messages_history_tests.rs`: `get_messages` (or a successor function) ordered by
  `seq`, with an `before_seq: Option<u64>` filter (`seq < before_seq`), returning enough data to build both
  `ChannelEntry::Message` and `ChannelEntry::UserJoined` (msg_type is already selected). Cover: page boundary
  has no duplicate/gap when paginating by the previous page's minimum seq; `None` returns the newest page.
- [ ] Task: Implement in `src/db/messages.rs`, replacing the `before` timestamp parameter. Keep the
  ascending/descending duality only if both call sites still need it after this refactor — check the subscribe
  path (it wants ASC) and REST (returns DESC today, but since the response becomes entries the frontend
  reverses anyway — pick one order, document it in the handler, and simplify the SQL to a single query if
  possible).

### 2.2 Service mapping and controller
- [ ] Task: The `msg_type → ChannelEntry` mapping currently exists twice (in `handle_subscribe` and in the
  frontend's `on_load_history`). Extract one backend pure function `to_channel_entry(ChatMessage) ->
  ChannelEntry` in `src/logic/` (unit tests: chat → Message, join → UserJoined carrying id/user/timestamp) and
  use it in both `handle_subscribe` and the REST path.
- [ ] Task: Update `get_channel_messages` (`src/channels.rs`): query param `before_seq: Option<u64>`, response
  `Json<Vec<ChannelEntry>>`. Update `tests/channel_messages_api_tests.rs` accordingly (assert on the
  `{"type": "chat", "data": {...}}` envelope).
- [ ] Task: Conductor — User Manual Verification 'Phase 2' (Protocol in workflow.md).

---

## Phase 3: Frontend follows the unified history API

- [ ] Task: TDD (frontend unit tests): `prepend_historical_messages` receiving `ChannelEntry` values directly
  (it already does — the change is upstream); add/adjust a test that the oldest loaded entry's `seq` is what
  gets passed to the next load request (pure helper extracting "oldest seq" from a message list — write it in
  `frontend/src/logic.rs` with tests first).
- [ ] Task: Update `fetch_historical_messages` (`frontend/src/main.rs`) to deserialize `Vec<ChannelEntry>` and
  take `before_seq: Option<u64>`; delete the msg_type mapping in `on_load_history`. Update the
  `on_load_history` callback signature (`Option<DateTime<Utc>>` → `Option<u64>`) through
  `chat_window.rs` props and `use_chat_scroll.rs` (find where the hook derives the "before" value from the
  oldest message — it must now use the seq helper).
- [ ] Task: Full frontend build + tests; run the messaging E2E spec (scrollback is covered there) against the
  stack.
- [ ] Task: Conductor — User Manual Verification 'Phase 3' (Protocol in workflow.md). Manual: fill a channel
  with >100 messages, scroll up repeatedly, verify no duplicates/gaps at page seams.

---

## Phase 4: JetStream replay replaces MessageStore

### 4.1 Replay helper
- [ ] Task: TDD (integration, pattern of `tests/nats_consumer_tests.rs`): new function in `src/logic/` (e.g.
  `replay.rs`): `replay_channel(state, channel_name, from_seq) -> Vec<ChannelEntry>` — ephemeral ordered
  consumer on `SANA`, `filter_subject = topic.<hex>`, `ByStartSequence(from_seq)`, drained up to the
  `last_sequence` captured from `stream.info()` before draining (stop immediately when `last_sequence <
  from_seq`, i.e. nothing to replay; also stop on a short timeout as a safety net). Entries get their `seq` set
  from the message info, like `handle_chat_message` does. Tests: replay from mid-stream returns exactly the
  tail; empty channel returns empty; `filter_subject` isolates channels.
- [ ] Task: Keep the drain loop's exit conditions in a small pure function (given `msg_seq`, `target_last_seq` →
  continue/stop) so the tricky boundary is unit-tested without NATS.

### 4.2 Rewire subscribe
- [ ] Task: In `handle_subscribe`: after DB history is loaded, call `replay_channel(state, channel,
  max_db_seq + 1)`; concatenate DB + replay entries; dedup by id (keep `merge_and_deduplicate` initially — it
  already does exactly this); apply the existing `should_send`/`last_seen_seq` filter; batch and send as today.
  Delete the `message_store.get_entries` branch.
- [ ] Task: Integration test (extend `tests/ws_persistence`/`ws_tests` patterns): publish messages, subscribe
  *before* the archiver persists them (use a stopped/slow durable, or simply don't start the archiver in the
  test), assert the subscriber receives them exactly once; then a mixed case with some rows already in the DB.

### 4.3 Rewire Lagged recovery
- [ ] Task: Change the forwarding task's Lagged back-fill (introduced by `durability_delivery_20260713`) to call
  `replay_channel(state, channel, last_forwarded_seq + 1)` instead of reading `MessageStore`. Adjust its tests.

### 4.4 Delete the store
- [ ] Task: Remove `MessageStore` and its constants from `src/messages.rs`, the `AppState` field, the
  `add_entry` call in `logic/nats.rs::handle_chat_message`, `merge_and_deduplicate` + `get_entries_after` if now
  unused, and the corresponding tests (`tests/state_tests.rs` / store-specific tests). Grep for
  `message_store` to catch stragglers. Zero clippy warnings — no `#[allow(dead_code)]` left behind.
- [ ] Task: Conductor — User Manual Verification 'Phase 4' (Protocol in workflow.md).

---

## Phase 5: Full verification

- [ ] Task: Full gate: `cargo test`, `cargo clippy -- -D warnings` (both crates), frontend build + tests, full
  E2E suite on the 2-replica stack (`cd e2e; npx playwright test --reporter=list`) — reconnect/resubscribe and
  scrollback specs are the sensitive ones. Manual two-browser check across replicas: send while the other
  client resubscribes; no lost or duplicated messages.
- [ ] Task: Update `conductor/tech-stack.md`'s DashMap/message-store wording (it mentions the in-memory message
  store) with a dated note pointing at this track.
- [ ] Task: Conductor — User Manual Verification 'Phase 5' (Protocol in workflow.md).
