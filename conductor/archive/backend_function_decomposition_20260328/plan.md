# Implementation Plan: Backend Function Decomposition

## Phase 1: State & Infrastructure [checkpoint: a029b59]
- [x] Task: Add `dashmap` dependency to root `Cargo.toml`
- [x] Task: Refactor `state.rs` to use `DashMap` and update `load_channels_from_db`
- [x] Task: Update call sites for channel maps to remove `.lock().await`
- [x] Task: Conductor - User Manual Verification 'State & Infrastructure' (Protocol in workflow.md)

## Phase 2: Core Logic Refactoring [checkpoint: c2ebd0d]
- [x] Task: Extract helper functions from `handle_subscribe()` in `ws_logic.rs`
- [x] Task: Extract helper functions from `process_and_publish_message()` in `ws_logic.rs`
- [x] Task: Introduce `WsContext` struct and refactor `decide()` signature
- [x] Task: Conductor - User Manual Verification 'Core Logic Refactoring' (Protocol in workflow.md)

## Phase 3: Utility & Idempotency Refactoring [checkpoint: 49a118d]
- [x] Task: Refactor `archiver.rs` (extract message loop body, move FK check to DB layer)
- [x] Task: Refactor `MessageStore::add_entry()` in `messages.rs`
- [x] Task: Conductor - User Manual Verification 'Utility & Idempotency Refactoring' (Protocol in workflow.md)

## Phase 4: Validation [checkpoint: 4db917a]
- [x] Task: Run `cargo check` and `cargo clippy` — fix all warnings
- [x] Task: Run `cargo test` — ensure no regressions
- [x] Task: Conductor - User Manual Verification 'Validation' (Protocol in workflow.md)
