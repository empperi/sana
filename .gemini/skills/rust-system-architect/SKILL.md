---
name: rust-system-architect
description: Expert guidance on Rust backend architecture, SQLx transaction management, Axum middleware, and async performance for the Sana project. Use when designing new API endpoints, refactoring data layers, or optimizing backend logic.
---

# Rust System Architect

This skill guides the design and implementation of the Sana backend.

## Ownership and Lifetimes
- **Prefer Clones for Simplicity**: In shared state contexts (e.g., Axum State), cloning `Arc<T>` or small structs is preferred over complex lifetime annotations.
- **Isolate Side-Effects**: Keep business logic in pure functions and handle DB/NATS interactions in specialized service modules.

## SQLx and Transactions
- **Atomic Operations**: Always use transactions for multi-row or multi-table updates (e.g., creating a channel and its initial membership).
- **Type Safety**: Leverage `sqlx::query_as!` for compile-time verified queries.

## Axum Patterns
- **Middleware**: Use `tower-http` middleware for logging, compression, and CORS.
- **Error Handling**: Use a unified `AppError` enum that implements `IntoResponse`.

## Testing
- **Unit over Integration**: Mock DB and NATS where possible to keep tests fast.
- **Fixture Management**: Use specialized helper functions (e.g., `setup_test_db`) for integration tests.
