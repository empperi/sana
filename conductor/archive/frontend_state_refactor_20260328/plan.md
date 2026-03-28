# Phase 1: Frontend State Refactor

## Objective
Eliminate the dual-state problem in `frontend/src/main.rs` where `UseState<ChatState>` and
`Rc<RefCell<ChatState>>` maintain parallel copies of the same data. Replace with a single global
state using Yew Context, aligned with the AGENTS.md mandate: "All UI state must be in one global
data structure called database."

## Background & Motivation
The current `main.rs` keeps state in two places:
- `chat_state: UseState<ChatState>` — the "real" state that triggers re-renders
- `state_ref: Rc<RefCell<ChatState>>` — a shadow copy shared across hooks and callbacks

These are synchronized manually via:
```rust
*state_ref.borrow_mut() = (**chat_state).clone();
```

This pattern creates divergence risk: if any code path mutates one but not the other, the UI
and the hooks see different state. Multiple hooks (`use_chat_websocket`, `use_channels`) access
`state_ref` independently — if they fire out of order, state can diverge.

## Key Files
- `frontend/src/main.rs` — Lines 45-53, state initialization and sync
- `frontend/src/logic.rs` — `ChatState` struct and mutations
- `frontend/src/hooks/use_chat_websocket.rs` — Consumes state_ref
- `frontend/src/hooks/use_channels.rs` — Consumes state_ref
- `frontend/src/components/chat_window.rs` — Reads chat_state

## Proposed Solution

### 1. Create a ChatStateContext provider

Create a new file `frontend/src/state.rs` implementing a context-based global store:

```rust
use yew::prelude::*;
use std::rc::Rc;

#[derive(Clone, PartialEq)]
pub struct ChatStateContext {
    pub state: ChatState,
    pub dispatch: Callback<ChatAction>,
}
```

Use `use_reducer` to hold state. The reducer accepts `ChatAction` variants (one per mutation) and
produces a new `ChatState`. Components read state via `use_context::<ChatStateContext>()`.

### 2. Define ChatAction enum

Extract all state mutations from `logic.rs` into an action enum:

```rust
pub enum ChatAction {
    HandleMessage(ChannelEntry),
    PrependHistory { channel_id: String, messages: Vec<ChatMessage> },
    SetChannels(Vec<Channel>),
    SelectChannel(String),
    SetConnected(bool),
    ClearSubscriptions,
    UpdateReadMarker { channel_id: String, message_id: String },
    // ... one variant per existing mutation
}
```

The reducer dispatches to the existing pure functions in `logic.rs`.

### 3. Wrap App in provider

In `main.rs`, wrap the component tree with the context provider:

```rust
html! {
    <ChatStateProvider>
        <App />
    </ChatStateProvider>
}
```

### 4. Refactor hooks to use context

Replace all `state_ref: Rc<RefCell<ChatState>>` parameters with:
```rust
let ctx = use_context::<ChatStateContext>().unwrap();
```

Hooks read `ctx.state` and mutate via `ctx.dispatch.emit(ChatAction::...)`.

### 5. Refactor callbacks in main.rs

The 31-line `on_create_channel` and 44-line `on_join_channel` callbacks (lines 87-196) should:
- Be extracted into `logic.rs` as functions
- Use `dispatch` instead of direct state mutation
- Deduplicate the shared channel-creation logic between them

## What NOT to change
- `logic.rs` pure functions remain as-is — the reducer delegates to them
- Component props and rendering logic stay the same initially
- WebSocketService internals are unchanged (it already uses callbacks)

## Implementation Steps

- [x] **Task: Create `frontend/src/state.rs` with `ChatAction` enum and reducer** 3a1b20a
- [x] **Task: Create `ChatStateProvider` component using `use_reducer`** 3a1b20a
- [x] **Task: Add `state.rs` to `frontend/src/lib.rs` module tree** 3a1b20a
- [x] **Task: Wrap root component with provider in `main.rs`** 3a1b20a
- [x] **Task: Refactor `use_chat_websocket` hook — replace `state_ref` with context** 3a1b20a
- [x] **Task: Refactor `use_channels` hook — replace `state_ref` with context** 3a1b20a
- [x] **Task: Refactor `main.rs` callbacks — extract to `logic.rs`, use dispatch** 3a1b20a
- [x] **Task: Remove `state_ref` and manual sync from `main.rs`** 3a1b20a
- [x] **Task: Run `cargo check --target wasm32-unknown-unknown` and fix any issues** 3a1b20a
- [x] **Task: Run existing frontend tests — ensure no regressions** 3a1b20a

## Verification
- Frontend compiles without warnings
- All existing `frontend/tests/logic_tests.rs` tests pass
- Manual test: channels load, messages send/receive, reconnection works
- Verify via Playwright: page loads, chat works end-to-end

## Phase: Review Fixes
- [x] Task: Apply review suggestions 5372c1f

## Risk & Rollback
This is a large refactor touching most frontend files. Mitigations:
- Do it in one PR to avoid a half-migrated state
- Keep `logic.rs` pure functions unchanged — only the wiring changes
- If broken, the git diff is fully revertible
