# Phase 5: Error Handling & Resilience

## Objective
Remove panic-on-failure paths, add timeouts to blocking operations, and improve resilience
of the system under degraded conditions (NATS down, DB slow, high message volume).

## Issues & Fixes

### 5a. Startup panics in `src/main.rs`

**Problem:** Lines 30, 43, 68 use `unwrap()` and `expect()` on NATS connection, stream creation,
and transaction begin. If any infrastructure is unavailable at startup, the process panics with
an unhelpful message.

**Fix:** Replace with proper error propagation. `main()` already returns `Result`, so use `?`
with context:
```rust
let nats_client = async_nats::connect(&config.nats_url)
    .await
    .context("Failed to connect to NATS")?;
```

Add `anyhow` to dependencies for `context()` in `main()` only. Library code should continue
using typed errors.

### 5b. Authentication session validation — no caching

**Problem:** `src/auth.rs` line 37 opens a database transaction on every single HTTP request to
validate the session cookie. At scale, this is a significant DB load.

**Fix:** Add a lightweight TTL cache for validated sessions:
- Use `mini-moka` or a simple `DashMap<SessionId, (User, Instant)>` with TTL
- Cache hit: return cached user if within TTL (e.g., 60 seconds)
- Cache miss: query DB, populate cache
- Logout: evict from cache

This avoids adding Redis as a dependency while still reducing DB pressure significantly.

### 5c. Missing channel membership check on message fetch

**Problem:** `src/channels.rs` line 158 `get_channel_messages()` does not verify the requesting
user is a member of the channel. Any authenticated user can read any channel's messages.

**Fix:** Add a membership check before returning messages:
```rust
let is_member = db::channels::is_channel_member(pool, user.id, channel_id).await?;
if !is_member {
    return Err(AppError::Forbidden);
}
```

Add `is_channel_member()` to `src/db/channels.rs`. This is a simple `SELECT EXISTS` query.

### 5d. WebSocket reconnection — unbounded retries

**Problem:** `frontend/src/services/websocket.rs` lines 62-104 loop forever with 2-second
backoff. If the server is permanently down, the client retries indefinitely, consuming
resources.

**Fix:** Implement capped exponential backoff:
- Start at 2s, double each attempt, cap at 30s
- After N total attempts (e.g., 20), stop retrying and show "connection lost" in UI
- Allow manual reconnect via user action

### 5e. NATS publish errors silently ignored

**Problem:** Several places publish to NATS without checking the result:
- `src/channels.rs` line 86-88: channel creation event
- `src/channels.rs` line 140-142: join event
- `src/logic/ws_logic.rs` line 272-275: chat message

**Fix:** Log warnings on publish failures. For chat messages, return an error to the client
via STOMP ERROR frame so the frontend can show "message not sent" and retry.

### 5f. In-memory message store — unbounded growth

**Problem:** `src/messages.rs` MessageStore trims to 100 entries per channel, but if the
archiver falls behind or fails, messages accumulate across many channels without a global
memory limit.

**Fix:** Add a global entry limit (e.g., 10,000 total entries across all channels). When
exceeded, evict oldest entries from the largest channel. Log a warning when eviction occurs
so operators know the archiver may be behind.

### 5g. `logic/nats.rs` — unwrap on stream lookup

**Problem:** Line 9 calls `jetstream.get_stream("SANA").await.unwrap()`. If the stream doesn't
exist at startup, the consumer task panics.

**Fix:** Use retry loop or propagate error gracefully. The stream should be created in `main.rs`
before spawning the consumer, but defensive coding should handle the race.

### 5h. Frontend async tasks — no error handling

**Problem:** Multiple places in frontend components spawn async tasks without catching errors:
- `components/profile_menu.rs` line 53: logout ignores server failure
- `hooks/use_auth_check.rs` line 16: no timeout on auth check
- `hooks/use_channels.rs` line 28: no timeout on channel fetch

**Fix:** Add timeout (e.g., 10s) to all frontend HTTP requests. On failure, show appropriate
UI feedback (toast notification or redirect) rather than silently failing.

## Implementation Steps

1. Add `anyhow` dependency, fix `main.rs` startup error handling
2. Add channel membership check in `get_channel_messages()`
3. Implement session cache with TTL in `auth.rs`
4. Add NATS publish error logging/handling
5. Cap WebSocket reconnection retries with exponential backoff
6. Add global memory limit to MessageStore
7. Fix `logic/nats.rs` stream lookup error handling
8. Add timeouts to frontend HTTP requests

## Verification
- `cargo test` — all tests pass
- Manual test: stop NATS, verify backend starts gracefully with retries
- Manual test: stop DB, verify backend logs error and retries
- Manual test: disconnect network, verify frontend shows "connection lost" after max retries
- Verify no `unwrap()` or `expect()` in non-test code (except where truly infallible)
