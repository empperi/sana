# Durability & Delivery Fixes — Specification

## Overview

The 2026-07-13 architecture review (`docs/architecture-review-2026-07-13.md`, findings B3/B5/B6/B7) identified
three defects that cause data loss or unbounded growth:

1. **B3 — Unbounded JetStream retention.** The `SANA` stream is created with no `max_age`/`max_bytes` limits, so
   every message and read marker is retained forever → guaranteed disk exhaustion.
2. **B5 — A message burst silently kills live delivery.** The per-subscription forwarding task exits on
   `broadcast::error::RecvError::Lagged` (buffer is 100), so the client stops receiving that channel while still
   believing it is subscribed.
3. **B7 — Server-side NATS consumer restarts lose live messages.** The fan-out consumer uses
   `DeliverPolicy::New`; when its message loop errors and the outer loop recreates it, everything published in
   between is never broadcast to that replica's connected clients.
4. **B6 — Missing index.** History queries filter on `channel_id` and sort by `created_at`, but no matching index
   exists → sequential scans that grow with total message volume.

## Requirements

### Stream retention limits

- The `SANA` stream gets configurable limits: `stream_max_age_hours` (default **720** = 30 days) and
  `stream_max_bytes` (default **8 GiB**), resolved through the existing `Config` precedence (env → config file →
  default). Env names: `STREAM_MAX_AGE_HOURS`, `STREAM_MAX_BYTES`.
- Because `get_or_create_stream` does not modify an existing stream, startup must also **update** the stream
  config so limits apply to deployments whose stream already exists.
- Constraint to preserve: the archiver resumes from `MAX(messages.seq)` in the DB. Its existing fallback (deliver
  `All` when the DB seq predates the stream's first sequence) already tolerates trimmed history; verify this with
  a test rather than assuming it.

### Lagged-subscriber recovery

- The forwarding task (`ws_logic::spawn_forwarding_task`) must distinguish `RecvError::Closed` (terminate) from
  `RecvError::Lagged` (recover and continue). It must never end silently while the socket is alive.
- Recovery: the task tracks the last stream `seq` it forwarded; on `Lagged` it back-fills from the in-memory
  `MessageStore` (entries with `seq` greater than the last forwarded one), logs a warning, and resumes live
  forwarding. (The store holds the most recent 100 entries per channel — the same size as the broadcast buffer —
  so a single lag event is fully recoverable; deeper gaps are bounded by raising the buffer, below.)
- The broadcast channel capacity is raised from 100 to **1024** (payloads are small strings; the memory cost is
  trivial and it makes lag events rare).
- Clients already deduplicate by message id, so overlap between back-fill and live delivery is harmless.

### Fan-out consumer gap-free restart

- The fan-out subscriber (`logic/nats.rs`) records the last stream sequence it processed (shared atomic in
  `AppState`). When the consumer is (re)created after the first time, it uses
  `DeliverPolicy::ByStartSequence(last + 1)` instead of `New`, so no messages are skipped across consumer
  restarts. First-ever creation keeps `New`.
- Redelivered duplicates are acceptable: `MessageStore::add_entry` is idempotent by id and clients dedupe by id.

### Index

- New migration: index on `messages (channel_id, created_at DESC)`.

## Acceptance criteria

1. After startup against a NATS server whose `SANA` stream pre-exists with unlimited retention, the stream's
   reported config shows the configured `max_age`/`max_bytes`.
2. A subscriber whose broadcast receiver lags (force with a small test buffer) receives every message exactly
   once (by id) and keeps receiving afterwards — verified by a unit/integration test.
3. Simulated consumer restart (drop and recreate with the recorded sequence) delivers the messages published
   during the gap.
4. `EXPLAIN` on the history query uses the new index (manual check is fine; the migration itself must apply
   cleanly on an existing database).
5. Existing archiver, WS, and E2E test suites stay green (E2E runs 2 replicas — cross-instance delivery must be
   unaffected).

## Out of scope

- Moving read markers out of the SANA stream (scaling track).
- Replacing `MessageStore` with JetStream replay (simplification track — note: that track will refactor the
  Lagged recovery introduced here to use JetStream replay instead of the in-memory store).
- Switching history ordering/pagination from `created_at` to `seq` (simplification track).
