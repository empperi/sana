# Implementation Plan — Scaling & Throughput Preparation

## Key Architecture Reference

- **Pool:** created in `src/db/mod.rs::connect` (`max_connections(5)` today). `Config` in `src/config.rs`.
- **Repositories** (`src/db/*.rs`) take `&mut Transaction<'_, Postgres>`; a few already take `&PgPool`
  (`get_all_channels`, `get_user_channels`, `search_unjoined_channels`) — that is the pattern to extend for
  read-only paths. sqlx queries run on `&PgPool` directly by binding `.fetch_*(pool)`.
- **Archiver:** `src/logic/archiver.rs` — durable pull consumer, `messages.next().await` one at a time,
  one transaction per message inside `handle_message` → `archive_*` functions.
- **Read markers today:** frontend sends a STOMP SEND with `message-type: read_marker`; `ws_logic::
  publish_read_marker` publishes a `ChannelEntry::ReadMarker` JSON to `topic.<hex>`; the fan-out consumer
  (`logic/nats.rs`) broadcasts it; the archiver persists it via `update_last_message_read_by_name`.
- **Attachments:** controller `src/attachments.rs` (buffers via `field.bytes()` and `tokio::fs::read`), logic in
  `src/logic/attachments.rs` (DB insert *before* file write — this ordering gets fixed here).
- **Multi-replica contract:** E2E runs 2 app replicas; read markers and messages must propagate across replicas.
- **Workflow rule:** new dependencies require updating `conductor/tech-stack.md` *before* implementing.

---

## Phase 1: Pool sizing and transaction-free reads (B14)

- [ ] Task: TDD in `tests/config_tests.rs`: add `db_max_connections: u32` (default 20, env
  `DB_MAX_CONNECTIONS`) to `Config`; use it in `db::connect`.
- [ ] Task: Convert the hot single-statement reads to pool-based execution. For each function, either change its
  signature to `&PgPool` (when no caller needs it inside a transaction) or add a `_pool` variant, following the
  existing pool-based functions in `db/channels.rs`. Minimum set: `users::get_user_by_id` as used by session
  validation and `/me` (`src/state.rs` / `src/auth.rs`), `channels::get_channel_by_name` as used by
  `ws_logic::resolve_channel_id`, `messages::get_last_message_read` (subscribe path in `ws_logic`),
  `channels::is_channel_member`, and the attachment metadata reads in `logic/attachments.rs` /
  `ws_logic::process_and_publish_message`. Remove the now-unneeded `begin()` calls at those call sites.
- [ ] Task: Update the affected tests (they call the repository functions — mirror the signature changes).
  `cargo test` green, clippy clean.
- [ ] Task: Conductor — User Manual Verification 'Phase 1' (Protocol in workflow.md).

---

## Phase 2: bcrypt via spawn_blocking (B15)

- [ ] Task: In `src/auth.rs`, wrap the `hash(...)` call in `register` and the `verify(...)` call in `login` with
  `tokio::task::spawn_blocking`, mapping `JoinError` through the existing `internal_error` helper. Ownership
  note: pass owned `String`s into the closure.
- [ ] Task: Existing auth integration tests must still pass (they cover both endpoints); no new tests needed
  beyond compilation and the suite. Clippy clean.
- [ ] Task: Conductor — User Manual Verification 'Phase 2' (Protocol in workflow.md).

---

## Phase 3: Batched archiver

- [ ] Task: TDD the pure part first (`tests/archiver_tests.rs`): a function that takes a batch of parsed
  `(channel_name, sequence, ChannelEntry)` items and groups them for persistence (messages+joins vs read
  markers), preserving order. Keep parsing (`payload` → entry, subject → channel name) reusable — extract the
  per-message decode currently inlined in `handle_message` into a pure function with unit tests (invalid
  payloads yield an "ack and skip" marker, preserving today's poison-message behavior).
- [ ] Task: Rework the consume loop in `archiver.rs`: replace the `messages()` stream with a loop over
  `consumer.batch().max_messages(200).expires(Duration::from_secs(2)).messages().await` (async-nats pull
  consumer batch API). Collect the fetched messages, decode them, persist the whole batch in **one**
  transaction (loop the existing `insert_message_with_fk_check` / marker updates inside it — multi-row VALUES is
  an optional refinement, not required), commit, then ack every message.
- [ ] Task: Fallback path: if the batch transaction fails, process that batch's messages individually with the
  existing per-message functions so a single poison message cannot wedge the stream. Log which message failed.
- [ ] Task: Integration test: publish ~500 messages across two channels, wait, assert all are in the DB exactly
  once and in seq order. Reuse the harness patterns from the existing archiver tests (unique durable names per
  test — see `start_with_durable`).
- [ ] Task: Conductor — User Manual Verification 'Phase 3' (Protocol in workflow.md).

---

## Phase 4: Read markers to their own stream

- [ ] Task: Stream setup: alongside the `SANA` stream creation in `src/main.rs` (or the setup module extracted by
  the durability track), create/update stream `SANA_READS`: subjects `["read.>"]`,
  `max_msgs_per_subject: 1`, `max_age: 24h`. Integration test asserting the stream config, like the SANA limits
  test.
- [ ] Task: Publishing: change `ws_logic::publish_read_marker` to publish to `read.<hex-channel>.<user_id>`
  (reuse `nats_util::encode`). The payload stays the same `ChannelEntry::ReadMarker` JSON.
- [ ] Task: Fan-out: add a second consumer task in `src/logic/nats.rs` (same retry-loop pattern as
  `start_nats_subscriber`, ordered consumer on `SANA_READS`) that parses the subject
  (`read.<hex>.<uuid>` → channel name), and broadcasts the payload through `state.channels` exactly as
  `handle_chat_message` does for markers today (no `MessageStore` involvement — markers were never stored
  there). Extract a shared subject-parsing pure function with unit tests (`tests/nats_util_tests.rs` or a new
  file): `read.<hex>.<uuid>` → `(channel_name, user_id)`, rejecting malformed subjects.
- [ ] Task: Archiving: add a second durable consumer (`postgres-archiver-reads`) in `archiver.rs` whose handler
  calls `update_last_message_read_by_name` (one transaction per marker is fine here — volume is compacted by
  `max_msgs_per_subject`). Ack semantics as in the main archiver.
- [ ] Task: Leave the legacy `topic.>` ReadMarker handling in `nats.rs`/`archiver.rs` untouched (it drains any
  in-flight markers during rollout; it can be deleted in a later cleanup).
- [ ] Task: Start both new consumers from `main.rs`. Integration test: publish a marker on the new subject,
  assert the DB row updates and a broadcast subscriber receives the ReadMarker entry.
- [ ] Task: E2E check (existing messaging spec covers read/unread indicators): run the full 2-replica E2E suite —
  marker set by a client on replica A must still reach a client on replica B. No frontend changes are expected;
  if any E2E test fails, the backend port is wrong, not the test.
- [ ] Task: Conductor — User Manual Verification 'Phase 4' (Protocol in workflow.md).

---

## Phase 5: Streaming attachments (B12)

- [ ] Task: Update `conductor/tech-stack.md`: add `tokio-util` (with the `io` feature) for streaming file IO,
  dated note referencing this track. Then add the dependency to `Cargo.toml`.
- [ ] Task: Upload: rework `logic/attachments.rs::upload_attachment` to accept the multipart `Field` (or an
  abstraction over a chunk stream, whichever keeps the function testable) instead of `Bytes`. Stream chunks to
  the destination file with `tokio::io::AsyncWriteExt`, counting bytes; if the count exceeds
  `max_attachment_size_bytes`, stop, delete the partial file, return the existing BadRequest error. Write the
  file **first**, then insert the DB row; delete the file if the insert fails.
- [ ] Task: Download: in `src/attachments.rs::download_attachment`, open the file with `tokio::fs::File`, wrap in
  `tokio_util::io::ReaderStream`, respond with `axum::body::Body::from_stream`, keeping the existing
  Content-Type/Content-Disposition headers. Missing file on disk still maps to the Internal error path.
- [ ] Task: Tests: existing `tests/attachment_api_tests.rs` (and the 3 MB test from the quick-wins track) must
  pass unchanged; add a test that an upload exceeding the limit returns BadRequest *and* leaves no file in the
  storage dir and no row in `attachments`.
- [ ] Task: Conductor — User Manual Verification 'Phase 5' (Protocol in workflow.md). Manual: upload/download a
  ~40 MB file through the browser while watching process memory.

---

## Phase 6: Full verification

- [ ] Task: Full gate: `cargo test`, `cargo clippy -- -D warnings` (both crates), frontend build, full E2E suite
  on the 2-replica stack (`cd e2e; npx playwright test --reporter=list`).
- [ ] Task: Conductor — User Manual Verification 'Phase 6' (Protocol in workflow.md).
