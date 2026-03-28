# Implementation Plan: Backend Function Decomposition

## Phase 1: State & Infrastructure
- [ ] Task: Add `dashmap` dependency to root `Cargo.toml`
- [ ] Task: Refactor `state.rs` to use `DashMap` and update `load_channels_from_db`
- [ ] Task: Update call sites for channel maps to remove `.lock().await`
- [ ] Task: Conductor - User Manual Verification 'State & Infrastructure' (Protocol in workflow.md)

## Phase 2: Core Logic Refactoring
- [ ] Task: Extract helper functions from `handle_subscribe()` in `ws_logic.rs`
- [ ] Task: Extract helper functions from `process_and_publish_message()` in `ws_logic.rs`
- [ ] Task: Introduce `WsContext` struct and refactor `decide()` signature
- [ ] Task: Conductor - User Manual Verification 'Core Logic Refactoring' (Protocol in workflow.md)

## Phase 3: Utility & Idempotency Refactoring
- [ ] Task: Refactor `archiver.rs` (extract message loop body, move FK check to DB layer)
- [ ] Task: Refactor `MessageStore::add_entry()` in `messages.rs`
- [ ] Task: Conductor - User Manual Verification 'Utility & Idempotency Refactoring' (Protocol in workflow.md)

## Phase 4: Validation
- [ ] Task: Run `cargo check` and `cargo clippy` — fix all warnings
- [ ] Task: Run `cargo test` — ensure no regressions
- [ ] Task: Conductor - User Manual Verification 'Validation' (Protocol in workflow.md)
