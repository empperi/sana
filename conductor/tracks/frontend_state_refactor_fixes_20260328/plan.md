# Implementation Plan: Frontend State Refactor — Review Fixes

## Phase 1: Logic Refactoring
- [x] Task: Extract `finalize_channel_join` helper in `frontend/src/logic.rs`
- [x] Task: Refactor `create_channel` to use helper and early returns
- [x] Task: Refactor `join_channel` to use helper and early returns
- [x] Task: Conductor - User Manual Verification 'Logic Refactoring' (Protocol in workflow.md)

## Phase 2: Performance & Style Fixes
- [x] Task: Move `messages` access inside effect body in `use_chat_websocket.rs`
- [x] Task: Add `Debug` to `ChatAction` and fix `"General"` comparison in `logic.rs`
- [x] Task: Fix line length violations in `main.rs` and `logic.rs`
- [x] Task: Conductor - User Manual Verification 'Performance & Style Fixes' (Protocol in workflow.md)

## Phase 3: Validation
- [x] Task: Run `cargo check --target wasm32-unknown-unknown` and fix any warnings
- [x] Task: Run existing frontend tests
- [x] Task: Conductor - User Manual Verification 'Validation' (Protocol in workflow.md)
