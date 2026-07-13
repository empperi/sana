# Implementation Plan — Durability & Delivery Fixes

## Key Architecture Reference

- **Stream setup:** `src/main.rs` creates the `SANA` stream (subjects `topic.>`) via `get_or_create_stream`.
  `Config` lives in `src/config.rs` (env → config file → default via `get_value`).
- **Fan-out consumer:** `src/logic/nats.rs::start_nats_subscriber` — outer retry loop, ordered pull consumer with
  `DeliverPolicy::New`, dispatches into `MessageStore` and per-channel `tokio::broadcast` senders held in
  `AppState.channels` (`src/state.rs`).
- **Forwarding tasks:** `src/logic/ws_logic.rs::spawn_forwarding_task` — one task per (socket, channel), reads the
  broadcast receiver, writes STOMP MESSAGE frames into the socket's mpsc sender.
- **Broadcast channels** are created in three places with capacity 100: `state.rs::load_channels_from_db` (twice)
  and `ws_logic.rs::get_or_create_broadcast_channel`. Keep the capacity as a single named constant.
- **Archiver:** `src/logic/archiver.rs` — durable pull consumer, resumes from `MAX(seq)` in the DB, falls back to
  `DeliverPolicy::All` when the stream was trimmed past it.
- **Tests:** `tests/nats_consumer_tests.rs`, `tests/archiver_tests.rs`, `tests/ws_logic_tests.rs` show the
  existing patterns (they use the real NATS/Postgres from docker-compose). Pure decision logic should be
  extracted into functions testable without infrastructure.

---

## Phase 1: Stream retention limits (B3)

### 1.1 Config values
- [ ] Task: TDD in `tests/config_tests.rs`: add `stream_max_age_hours: u64` (default 720) and
  `stream_max_bytes: i64` (default 8 GiB) to `Config`, parsed via the existing `get_value` + `parse` pattern
  (see `max_attachment_size_bytes`).

### 1.2 Apply limits at startup
- [ ] Task: Extract the stream-config construction in `src/main.rs` into a small pure function (e.g. in a new
  `src/logic/stream_setup.rs` or `nats_util.rs`) that maps `&Config` → `jetstream::stream::Config` with
  `max_age` (from hours) and `max_bytes` set. Unit-test the mapping (a pure function — no NATS needed).
- [ ] Task: In `src/main.rs`, after `get_or_create_stream`, call `jetstream.update_stream(&config)` so
  pre-existing streams adopt the limits. Treat an update error as fatal at startup (same `context(...)` style as
  the surrounding code). Note: `get_or_create_stream` + unconditional `update_stream` is simpler than comparing
  configs — updating to identical values is a no-op on the server.
- [ ] Task: Integration test (pattern of `tests/nats_consumer_tests.rs`, use a uniquely-named test stream, not
  `SANA`): create a stream without limits, run the setup function, assert the fetched stream info shows the
  configured `max_age`/`max_bytes`.

### 1.3 Archiver trim tolerance
- [ ] Task: Add a test to `tests/archiver_tests.rs` asserting the deliver-policy decision when the DB max seq is
  *older* than the stream's first sequence (the trimmed case) resolves to `DeliverPolicy::All`. The decision
  logic is currently inline in `archiver::start_with_durable` — extract it into a pure function
  `choose_deliver_policy(first_stream_seq, last_db_seq) -> DeliverPolicy` so this is a plain unit test, and have
  `start_with_durable` call it.

### 1.4 Conductor — User Manual Verification 'Phase 1'
- [ ] Task: Conductor — User Manual Verification 'Phase 1' (Protocol in workflow.md). Include a manual check:
  `curl http://localhost:8222/jsz?streams=true` (NATS monitoring) shows the limits on a locally running stack.

---

## Phase 2: Lagged-subscriber recovery (B5)

### 2.1 Extract and test the back-fill decision
- [ ] Task: TDD in `tests/ws_logic_tests.rs`: a pure function that, given the entries currently in the
  `MessageStore` for a channel and the last forwarded seq, returns the entries to back-fill (those with
  `Some(seq) > last_forwarded`, in seq order; entries without seq are skipped). Write the tests first, then the
  function in `ws_logic.rs`.

### 2.2 Rework the forwarding loop
- [ ] Task: Change `spawn_forwarding_task` to take `&AppState` (it needs the `MessageStore`) and to loop on
  `rx.recv().await` matching three cases: `Ok(msg)` → parse (it already parses for the seq header), remember the
  seq, forward; `Err(Lagged(n))` → `tracing::warn!`, back-fill via the Phase 2.1 function (formatting frames the
  same way as normal forwarding), continue; `Err(Closed)` → break. Update the call sites
  (`handle_subscribe`, `handle_system_subscribe` — the system channel has no seqs, so back-fill yields nothing
  there, which is fine).
- [ ] Task: Integration-style test: create a broadcast channel with a deliberately tiny capacity, a populated
  `MessageStore`, send more messages than the capacity while the consumer is slow, and assert the mpsc receiver
  ends up with every message id exactly once and the task is still alive (send one more message and see it
  arrive).

### 2.3 Raise broadcast capacity
- [ ] Task: Introduce a `const BROADCAST_CAPACITY: usize = 1024;` (e.g. in `state.rs`, re-used by
  `ws_logic::get_or_create_broadcast_channel`) and replace the three hard-coded `100`s.

### 2.4 Conductor — User Manual Verification 'Phase 2'
- [ ] Task: Conductor — User Manual Verification 'Phase 2' (Protocol in workflow.md).

---

## Phase 3: Gap-free fan-out consumer restart (B7)

### 3.1 Track the last processed sequence
- [ ] Task: Add `last_fanout_seq: Arc<AtomicU64>` to `AppState` (0 = nothing processed yet). In
  `logic/nats.rs::handle_nats_message`, store each message's `stream_sequence` after processing.

### 3.2 Restart from the recorded sequence
- [ ] Task: Extract a pure function `fanout_deliver_policy(last_seq: u64) -> DeliverPolicy` (`New` when 0,
  `ByStartSequence { start_sequence: last_seq + 1 }` otherwise) with unit tests, and use it where
  `start_nats_subscriber` builds the `OrderedConfig`.
- [ ] Task: Integration test (pattern of `tests/nats_consumer_tests.rs`): publish, process, record seq; publish
  two more messages *without* a consumer; recreate the consumer with the recorded seq and assert both gap
  messages are delivered. Duplicates are acceptable — assert on set membership by id, not counts.

### 3.3 Conductor — User Manual Verification 'Phase 3'
- [ ] Task: Conductor — User Manual Verification 'Phase 3' (Protocol in workflow.md).

---

## Phase 4: Index migration and full verification

- [ ] Task: New migration: `CREATE INDEX IF NOT EXISTS idx_messages_channel_created ON messages
  (channel_id, created_at DESC);`. Verify `cargo test` applies it (the test DB runs migrations on startup).
- [ ] Task: Full gate: `cargo test`, `cargo clippy -- -D warnings`, frontend build, then the full E2E suite
  against the 2-replica stack (`docker compose -f docker-compose.e2e.yml --project-name sana-e2e up --build
  --wait`, then `cd e2e; npx playwright test --reporter=list`). Messaging across replicas must be unaffected.
- [ ] Task: Conductor — User Manual Verification 'Phase 4' (Protocol in workflow.md).
