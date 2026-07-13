# Scaling & Throughput Preparation — Specification

## Overview

The 2026-07-13 architecture review (`docs/architecture-review-2026-07-13.md`, §3 and findings B12/B14/B15)
identified the concrete throughput ceilings the system hits first: the sequential one-transaction-per-message
archiver, the 5-connection DB pool with a transaction opened for every read, read markers riding the durable
chat stream, blocking bcrypt on the async runtime, and whole-file memory buffering for attachments. This track
removes those ceilings without changing the architecture's shape.

**Ordering:** run after `durability_delivery_20260713` (it touches the same stream setup and archiver code).

## Requirements

### 1. DB pool sizing and transaction-free reads (B14)

- Pool size becomes configurable: `db_max_connections` (env `DB_MAX_CONNECTIONS`), default **20**.
- Single-statement, read-only paths stop opening explicit transactions and execute directly against the pool.
  The architecture rule "the service layer is the only place transactions are opened" still holds — these reads
  simply don't need one. In scope at minimum: session validation, `resolve_channel_id`, `get_last_message_read`
  during subscribe, membership checks, attachment metadata reads, and `/api/auth/me`'s user fetch. Multi-statement
  or write flows keep their transactions.

### 2. bcrypt off the async runtime (B15)

- `hash` and `verify` run inside `tokio::task::spawn_blocking` in register/login. Behavior is otherwise
  unchanged.

### 3. Batched archiver

- The archiver consumes in batches (up to **200** messages or a short expiry, whichever comes first) and
  persists each batch in a **single transaction**, acking all messages only after commit.
- Poison-message safety is preserved: if a batch transaction fails, fall back to processing that batch's
  messages one by one with the existing per-message semantics (permanent failures are logged and acked, as
  today). Redelivery after a crash is safe because inserts are `ON CONFLICT DO NOTHING` and read-marker updates
  are idempotent.

### 4. Read markers leave the chat stream

- New JetStream stream `SANA_READS` with subjects `read.>`. Read markers are published to
  `read.<hex-channel>.<user_id>` instead of `topic.<hex-channel>`.
- Compaction by design: `max_msgs_per_subject = 1` (only the latest marker per user+channel is retained) plus
  `max_age` of 24 h. This removes the unbounded per-marker growth and takes markers out of the chat stream's
  sequence space.
- The read-your-own-writes rule is preserved: markers still go through NATS and are processed when they come
  back. A second fan-out consumer (mirroring the existing one) broadcasts `ChannelEntry::ReadMarker` to the same
  in-process channel broadcasts, so the **wire format to clients and the frontend are completely unchanged**. A
  second durable archiver consumer (`postgres-archiver-reads`) performs the DB updates.
- The old code path (ReadMarker entries arriving on `topic.>`) keeps working during the transition — leave the
  existing handling in place; it simply stops receiving traffic.

### 5. Streaming attachments (B12)

- Upload: the multipart field is streamed to disk chunk-by-chunk (no full buffering); the size limit is enforced
  while streaming (abort + delete the partial file when exceeded). Order is corrected: file is written first,
  then the DB row inserted; on DB failure the file is deleted — no more DB rows pointing at missing files.
- Download: the response streams from disk (`ReaderStream` → `Body::from_stream`) instead of `tokio::fs::read`.
- This adds the `tokio-util` dependency; per workflow, `conductor/tech-stack.md` must be updated before
  implementation.

## Acceptance criteria

1. Config test proves pool size is configurable; a grep of the hot read paths shows no `begin()` for
   single-statement reads.
2. Archiver integration test: publish N > batch-size messages, all end up in the DB; kill/restart mid-stream and
   verify no loss and no duplicates (idempotent inserts).
3. Read markers: marking read in client A propagates to client B connected to the **other replica** (E2E,
   2-replica stack); the `SANA` stream receives no `read_marker` entries anymore; `SANA_READS` info shows
   `max_msgs_per_subject = 1`.
4. A 40 MB upload and download complete without the process RSS growing by the file size (manual observation is
   acceptable); an oversized upload leaves no partial file and no DB row.
5. Full backend suite, clippy (zero warnings), and E2E suite green.

## Out of scope

- Object storage (S3/MinIO) for attachments — the shared-volume limitation remains; this track only fixes memory
  behavior.
- Per-channel filtered consumers / interest-based fan-out.
- Shared session cache, rate limiting, message-length limits.
- Removing the in-memory `MessageStore` (simplification track).
