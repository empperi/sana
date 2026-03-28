# Phase 2: Backend Function Decomposition

## Objective
Break up oversized functions, remove nested control flow, and replace contended locks with
appropriate concurrent data structures. This phase targets AGENTS.md violations: functions over
15 lines, nested if/match blocks, and unnecessary mutability.

## Key Files & Issues

### 2a. `src/logic/ws_logic.rs` — Most critical

**`handle_subscribe()` (lines 88-138, 51 lines)**

Currently does 5 things in one function:
1. Fetches historical messages from DB (lines 97-107)
2. Fetches recent messages from in-memory store (lines 121-126)
3. Deduplicates across both sources (line 101, 113 — `should_send()` predicate)
4. Combines into ordered list
5. Sends in batches of 20 (lines 131-135)

**Proposed decomposition:**
```
handle_subscribe()
├── fetch_db_history(pool, channel_id, last_seen_seq) -> Vec<ChannelEntry>
├── fetch_inmemory_recent(message_store, channel_id, last_seen_seq) -> Vec<ChannelEntry>
├── merge_and_deduplicate(db_entries, mem_entries) -> Vec<ChannelEntry>
└── send_in_batches(sender, entries, batch_size=20) -> Result<()>
```

Each extracted function should be pure (except `send_in_batches`) and independently testable.

**`process_and_publish_message()` (lines 214-283, 70 lines)**

Currently does:
1. Parses message content (lines 218-223)
2. Resolves channel ID from destination (lines 225-257, 33 lines with 3 levels of nesting)
3. Constructs ChannelEntry (lines 259-270)
4. Publishes to NATS (lines 272-275)

**Proposed decomposition:**
```
process_and_publish_message()
├── resolve_channel_id(state, destination) -> Result<Uuid>
├── build_channel_entry(user, channel_id, content, msg_type) -> ChannelEntry
└── publish_to_nats(jetstream, channel_id, entry) -> Result<()>
```

`resolve_channel_id()` should extract the nested match/lock/lookup chain into a flat function
with early returns.

**`decide()` function (lines 63-86)**

7 parameters in the signature. Should accept a struct:
```rust
pub struct WsContext<'a> {
    pub pool: &'a PgPool,
    pub jetstream: &'a async_nats::jetstream::Context,
    pub state: &'a AppState,
    pub user: &'a User,
    pub raw_msg: &'a str,
}
```

### 2b. `src/logic/archiver.rs` — Multiple long functions

Functions in the 30-40 line range handling JetStream consumption and persistence.
Key extractions:
- `is_foreign_key_violation()` (lines 293-299) should move to `src/db/mod.rs` as shared utility
- Consumer setup logic (lines 21-33) should be its own function
- Message processing loop body should be extracted into `process_single_message()`

### 2c. `src/messages.rs` — `MessageStore::add_entry()`

`add_entry()` (lines 73-103, 31 lines) with nested match for idempotency:

**Proposed decomposition:**
```
add_entry()
├── find_existing_entry(entries, message_id) -> Option<usize>
├── update_existing(entries, index, new_entry)
└── insert_and_trim(entries, new_entry, max_size=100)
```

The O(n) linear search for idempotency (line 85-93) should be noted as a future optimization
target (HashSet for dedup), but is acceptable at current scale.

### 2d. `src/state.rs` — Lock contention

Replace:
```rust
pub channels: Arc<Mutex<HashMap<String, broadcast::Sender<String>>>>
pub channel_ids: Arc<Mutex<HashMap<String, Uuid>>>
```

With:
```rust
pub channels: DashMap<String, broadcast::Sender<String>>
pub channel_ids: DashMap<String, Uuid>
```

`DashMap` provides fine-grained per-shard locking, eliminating contention when different
channels are accessed concurrently. Add `dashmap` to `Cargo.toml` dependencies.

Also update `load_channels_from_db()` which currently locks both mutexes sequentially.

### 2e. Nested control flow cleanup

**`ws_logic.rs` channel ID resolution (lines 225-257):**
```rust
// Before (3 levels of nesting):
if let Some(channel) = destination.strip_prefix("/topic/") {
    let ids = state.channel_ids.lock().await;
    if let Some(id) = ids.get(channel) {
        // ...
    } else {
        // ...
    }
}

// After (flat with early returns):
let channel = destination.strip_prefix("/topic/")
    .ok_or(WsError::InvalidDestination)?;
let channel_id = state.channel_ids.get(channel)
    .ok_or(WsError::ChannelNotFound(channel.to_string()))?;
```

## Implementation Steps

1. Add `dashmap` dependency to root `Cargo.toml`
2. Refactor `state.rs` — replace `Arc<Mutex<HashMap>>` with `DashMap`
3. Update all call sites that `.lock().await` on channel maps
4. Extract `ws_logic.rs` helper functions (fetch_db_history, resolve_channel_id, etc.)
5. Introduce `WsContext` struct for `decide()` parameters
6. Extract `archiver.rs` helpers (process_single_message, move FK check to db layer)
7. Extract `messages.rs::add_entry()` helpers
8. Run `cargo check` and `cargo clippy` — fix all warnings
9. Run `cargo test` — ensure no regressions

## What NOT to change
- Public API signatures of handler functions (would break router)
- NATS subject naming conventions
- Database queries or schema
- The read-your-own-writes pattern

## Verification
- `cargo check` — zero warnings
- `cargo clippy` — zero warnings
- `cargo test` — all existing tests pass
- No function exceeds 15 lines (except documented exceptions)
- No nesting deeper than 2 levels in refactored functions
