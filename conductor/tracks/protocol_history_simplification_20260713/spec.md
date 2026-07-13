# Protocol Documentation & History Simplification — Specification

## Overview

The 2026-07-13 architecture review (`docs/architecture-review-2026-07-13.md`, §1.3, §1.4, §4.2, finding B10)
recommended three strategic simplifications. They reduce the trickiest code in the repo and prepare the wire
protocol for future native mobile clients.

1. **Document the wire protocol.** The STOMP dialect is custom (non-spec headers, JSON envelopes, no
   content-length). Undocumented, it blocks any second client implementation.
2. **Unify the two history paths.** Initial history arrives over STOMP as `ChannelEntry::Batch`; scrollback
   arrives over REST as `Vec<ChatMessage>` that the frontend re-wraps into `ChannelEntry`. The REST endpoint also
   orders and paginates by `created_at`, which is per-replica wall-clock time and can misorder relative to the
   authoritative stream `seq` (finding B10).
3. **Replace the in-memory `MessageStore` with JetStream replay.** The store exists only to cover the archiver
   lag between "published to NATS" and "visible in Postgres". JetStream itself already retains those messages;
   replaying from the stream removes the store, the merge/dedup logic in `handle_subscribe`, and the global
   eviction heuristics.

**Ordering:** run after `durability_delivery_20260713` (retention limits + Lagged recovery — the recovery
mechanism is refactored here) and ideally after `scaling_throughput_prep_20260713` (read markers off the chat
stream keep replay traffic clean).

## Requirements

### Wire protocol document

- New `docs/wire-protocol.md` describing, precisely enough for an independent client implementation:
  frame-by-frame reference (CONNECT/CONNECTED incl. `user_id`/`username` headers; SUBSCRIBE incl.
  `last_seen_seq` and `receipt`; SEND incl. `message_id` and `message-type: read_marker`; MESSAGE incl. the
  `seq` header; RECEIPT; ERROR incl. `receipt-id`/`message_id`), the `ChannelEntry` JSON schema for every
  variant, the subscribe/history/batch sequence, reconnection with `last_seen_seq`, and an explicit list of
  divergences from the STOMP 1.2 spec (no `content-length`, header value trimming, no escaping, single
  hard-coded subscription id). Documentation only — no behavior change.

### Unified, seq-ordered history

- `GET /api/channels/:id/messages` returns `Vec<ChannelEntry>` (chat messages and join events in their proper
  variants), ordered by `seq`, paginated with a `before_seq: Option<u64>` query parameter. The `before`
  timestamp parameter is removed (internal API; frontend is the only consumer and is updated in the same track).
- New index to support it: `messages (channel_id, seq DESC)`.
- The frontend consumes `ChannelEntry` directly (deleting its ChatMessage→ChannelEntry mapping) and passes the
  oldest loaded entry's `seq` when requesting older history.

### MessageStore replaced by JetStream replay

- On SUBSCRIBE, the gap between DB history and live broadcast is filled by replaying the channel's subject from
  JetStream: an ephemeral ordered consumer with `filter_subject = topic.<hex-channel>` and
  `DeliverPolicy::ByStartSequence(max_db_seq + 1)`, drained until it reaches the stream's current end, then
  discarded. Combined history (DB + replay) is deduplicated by entry id, filtered by the client's
  `last_seen_seq`, and sent in batches exactly as today.
- The Lagged-recovery back-fill introduced by `durability_delivery_20260713` switches from reading the
  `MessageStore` to the same replay helper (replay from `last_forwarded_seq + 1`).
- `MessageStore`, its `AppState` field, `add_entry` calls in the fan-out consumer, and
  `merge_and_deduplicate`/mem-history filtering in `handle_subscribe` are deleted.
- Behavior guarantee (unchanged from today, now via replay): a client subscribing immediately after messages
  were published — before the archiver persisted them — still receives those messages exactly once, on any
  replica.

## Acceptance criteria

1. `docs/wire-protocol.md` exists and matches the implementation (spot-check each frame against the parsers in
   `src/stomp.rs` / `frontend/src/stomp.rs`).
2. Scrollback pagination returns no duplicates and no gaps across page boundaries (seq-based test with
   controlled data).
3. Subscribe-during-archiver-lag test: publish messages, subscribe before the archiver has persisted them,
   receive them exactly once. Repeat with messages split between DB and stream.
4. `MessageStore` no longer exists in the codebase; `cargo clippy -- -D warnings` clean.
5. Full E2E suite green on the 2-replica stack, including scrollback and reconnect behaviors.

## Out of scope

- Changing the transport (JSON-over-WS instead of STOMP) — the doc captures the dialect as-is.
- Bearer-token auth and other mobile prerequisites (separate future track).
- Per-channel filtered consumers for the *live* fan-out (replay consumers here are per-subscribe and ephemeral).
