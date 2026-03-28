# Phase 2: Frontend State Refactor — Review Fixes

## Objective
Address issues found during review of the Phase 1 frontend state refactor. These are
targeted fixes to the code that was just refactored — not new features.

## Key Files
- `frontend/src/hooks/use_chat_websocket.rs` — unnecessary messages clone
- `frontend/src/logic.rs` — duplicated channel-join logic, nested control flow, minor style issues
- `frontend/src/main.rs` — line length violation in `fetch_historical_messages` signature

## Issues to Fix

### 2a. Remove unnecessary `messages` clone on every render

**File:** `frontend/src/hooks/use_chat_websocket.rs`, line 48

```rust
let messages = ctx.state.messages.clone();  // clones ALL message history every render
```

The `messages` HashMap is cloned on every render but only consumed inside `use_effect_with`
that depends on `(channels, status)`. This means every incoming message or state change
triggers a full deep-clone of all message history for nothing.

**Fix:** Move the `messages` access inside the effect body where it's actually needed, or
restructure to avoid the eager clone. The effect only needs `messages` to look up `last_seq`
for a channel — this lookup can happen inside the effect closure using a fresh context read.

### 2b. Flatten nested `if let` chains in `create_channel` / `join_channel`

**File:** `frontend/src/logic.rs`, lines 29-42 and 62-73

Both functions have three-level nesting that silently swallows errors:

```rust
if let Ok(r) = resp {           // network error? silently ignored
    if r.status() == 201 {       // non-201? silently ignored
        if let Ok(channel) = ... // parse error? silently ignored
```

**Fix:** Use early returns with `gloo_console::error!()` logging for each failure case.
Flatten the happy path to avoid nesting.

### 2c. Deduplicate shared channel-join logic

**File:** `frontend/src/logic.rs`, lines 32-39 vs 64-71

Both `create_channel` and `join_channel` contain the same 4-line block:
1. `dispatch.emit(ChatAction::JoinChannel(...))`
2. Subscribe via websocket
3. `dispatch.emit(ChatAction::AddSubscribedChannel(...))`
4. `on_success.emit(channel.name)`

**Fix:** Extract a shared helper function:
```rust
fn finalize_channel_join(
    channel: Channel,
    dispatch: &Callback<ChatAction>,
    ws_service: &Rc<RefCell<Option<Rc<WebSocketService>>>>,
    on_success: &Callback<String>,
)
```

### 2d. Minor style fixes

1. **Add `Debug` derive to `ChatAction`** (`logic.rs`, line 92) — `ChatState` already
   derives `Debug`, `ChatAction` should too for consistency and diagnostics.

2. **Remove `"General".to_string()` for comparison** (`logic.rs`, line 317):
   ```rust
   // Before:
   if !self.channels.contains(&"General".to_string()) {
   // After:
   if !self.channels.iter().any(|c| c == "General") {
   ```

3. **Wrap long lines** that exceed 120 characters:
   - `main.rs:22` — `fetch_historical_messages` function signature (158 chars)
   - `logic.rs:319` — `Uuid::parse_str` line (128 chars)
   - `use_chat_websocket.rs:29` — `WebSocketService::connect` call (128 chars)

## Implementation Steps

1. Extract `finalize_channel_join` helper in `logic.rs`
2. Refactor `create_channel` and `join_channel` to use the helper and flatten control flow
3. Move `messages` access inside the effect body in `use_chat_websocket.rs`
4. Add `Debug` to `ChatAction`, fix `"General"` comparison, wrap long lines
5. Run `cargo check --target wasm32-unknown-unknown` — zero warnings
6. Run `cargo test` in frontend — ensure no regressions

## What NOT to change
- The reducer, `ChatStateProvider`, or `ChatStateContext` — these are fine
- Component structure or props
- The `chat_app()` function structure — it's long but each callback is small and clear
- `render_app()` HTML lines — exempt from line length rule

## Verification
- `cargo check --target wasm32-unknown-unknown` — zero warnings
- All frontend tests pass
- No function exceeds 15 lines (except HTML structure definitions)
- No nesting deeper than 2 levels in refactored functions
