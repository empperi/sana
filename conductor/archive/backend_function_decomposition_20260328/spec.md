# Specification: Backend Function Decomposition

## Overview
Break up oversized functions, remove nested control flow, and replace contended locks with appropriate concurrent data structures. This targets AGENTS.md violations: functions over 15 lines, nested if/match blocks, and unnecessary mutability.

## Functional Requirements
- **Function Decomposition**: Split large functions in `ws_logic.rs`, `archiver.rs`, and `messages.rs` into smaller, focused, and testable units.
- **Lock Optimization**: Replace `Arc<Mutex<HashMap>>` with `DashMap` in `state.rs` to reduce lock contention.
- **Control Flow Flattening**: Replace deeply nested `if let`/`match` blocks with early returns.
- **Parameter Refactoring**: Use context structs instead of long parameter lists for complex handlers like `decide()`.

## Acceptance Criteria
- No refactored function exceeds 15 lines (with documented exceptions).
- No nesting deeper than 2 levels in refactored functions.
- `cargo check` and `cargo clippy` produce zero warnings.
- All existing tests pass.
- `DashMap` is used for channel state management.
