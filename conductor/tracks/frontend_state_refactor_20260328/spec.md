# Specification: Frontend State Refactor

## Objective
Eliminate the dual-state problem in `frontend/src/main.rs` where `UseState<ChatState>` and `Rc<RefCell<ChatState>>` maintain parallel copies of the same data. Replace with a single global state using Yew Context.

## Background
The current `main.rs` keeps state in two places:
- `chat_state: UseState<ChatState>` — the "real" state that triggers re-renders.
- `state_ref: Rc<RefCell<ChatState>>` — a shadow copy shared across hooks and callbacks.

Synchronization is performed manually via `*state_ref.borrow_mut() = (**chat_state).clone();`, which creates divergence risk.

## Requirements
1.  **Single Source of Truth:** All UI state must be in one global data structure.
2.  **Yew Context:** Use Yew Context to provide the state to all components.
3.  **Reducer Pattern:** Use `use_reducer` to manage state mutations via actions.
4.  **Refactor Hooks:** Update `use_chat_websocket` and `use_channels` to consume the context.
5.  **Clean Callbacks:** Extract logic from `main.rs` into `logic.rs`.

## Technical Architecture
1.  **ChatStateContext:** Define a new context in `frontend/src/state.rs`.
2.  **ChatAction:** Define an enum for all possible state mutations.
3.  **ChatStateProvider:** A wrapper component that provides the context.
4.  **Hooks Update:** Hooks will now call `use_context::<ChatStateContext>()` instead of taking `state_ref`.
