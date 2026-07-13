# Correctness Quick Wins — Specification

## Overview

A batch of small, independent correctness fixes from the 2026-07-13 architecture review
(`docs/architecture-review-2026-07-13.md`, findings B4/B8/B9/B11 plus P2 hygiene items). Each fix is
self-contained; together they remove the sharpest user-visible edges.

## Requirements

### 1. Attachment uploads above 2 MB must work (B4)

- Axum's default request-body limit (2 MB) currently caps uploads even though config and frontend advertise
  50 MB. Install a `DefaultBodyLimit` sized from `Config.max_attachment_size_bytes` plus a 1 MiB margin for
  multipart framing.
- Uploads exceeding the configured limit must still be rejected with the existing `AppError::BadRequest` message
  (the in-handler size check remains the authoritative one).

### 2. RECEIPT means "the publish succeeded" (B8)

- For `SEND` frames carrying a `receipt` header, the server currently emits `RECEIPT` unconditionally, even when
  the NATS publish fails (the client then receives both ERROR and RECEIPT). After this fix: `RECEIPT` is sent
  only after the publish (message or read marker) succeeds; on failure only `ERROR` is sent, carrying the
  `receipt-id`.
- `SUBSCRIBE` receipt behavior is unchanged (the frontend's reconnect sync depends on it).

### 3. Failed sends are visible to the user (B9)

- Server: `ERROR` frames caused by a failed message publish include a `message_id` header when the id is known.
- Frontend: an `ERROR` frame with a `message_id` marks the matching optimistic pending message as **failed**
  (new client-side flag on `ChatMessage`, like `pending`). Failed messages render visually distinct (e.g. red
  accent + "failed to send" note near the timestamp) instead of staying "pending" forever.
- Retry/resend UX is out of scope; the message simply shows as failed.

### 4. Input validation and correct conflict statuses (B11)

- Channel creation validates the name: trimmed; length 1–64 characters after trim; no control characters; the
  reserved name `system.channels` and any name starting with `system.` are rejected. Violations → `400` with a
  JSON error body (same shape as `auth.rs`'s `ErrorResponse`).
- Unique-constraint violations surface as `409 Conflict`, not `500`: duplicate channel name on create, duplicate
  username on register (keep the friendly pre-check; the 409 mapping covers the race).
- `GET /api/channels/:id/messages` validates `limit` to 1–1000 (400 otherwise) — currently only the upper bound
  is checked and `limit=0`/negatives reach SQL.

### 5. Dead code removal

- Delete `frontend/src/communication.rs` (superseded by `services/websocket.rs`; referenced nowhere).

## Acceptance criteria

1. Integration test: a 3 MB upload succeeds end-to-end; an upload over the configured max is rejected with the
   JSON error.
2. Integration/WS test: `SEND` with a `receipt` header to a failing publish path yields ERROR (with receipt-id)
   and no RECEIPT; a successful `SEND` with `receipt` yields RECEIPT.
3. Frontend unit tests: reducer marks the matching pending message failed on the new action; unknown ids are a
   no-op.
4. API tests: invalid channel names → 400; duplicate channel name → 409; duplicate username race → 409;
   `limit=0` → 400.
5. `cargo clippy -- -D warnings` clean in both crates; full E2E suite green (2-replica stack).

## Out of scope

- Message-length limits on SEND bodies and WS heartbeats (P2 items — separate work).
- Resend/retry UI for failed messages.
- Toast/notification system.
