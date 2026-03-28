# Architecture & Code Quality Plan

## Status
**Created:** 2026-03-28
**Status:** Draft — awaiting review

## Summary

A comprehensive review of the Sana codebase identified 6 major architectural issues, multiple coding guideline
violations, and significant testing gaps. This plan organizes the work into 5 phases, ordered by impact and risk.

## Findings at a Glance

| Area | Severity | Description |
|------|----------|-------------|
| Frontend dual state | Critical | `UseState` + `Rc<RefCell>` creates two sources of truth |
| Backend function decomposition | High | `ws_logic.rs` has 51-line and 70-line functions with deep nesting |
| Testing gaps | High | No tests for ws_logic, archiver, nats consumer, or frontend hooks |
| Error handling & resilience | Medium | Unwraps in startup, no timeouts on auth, unbounded reconnection |
| State concurrency | Medium | `Arc<Mutex<HashMap>>` under contention; should be `DashMap` or `RwLock` |
| Infrastructure hardening | Low | No TLS, no gzip, no persistent volumes, no resource limits |

## Phases

Each phase has a dedicated plan document:

1. **[Frontend State Refactor](01-frontend-state-refactor.md)** — Critical
   Eliminate the dual-state problem. Single global state via Yew Context.

2. **[Backend Function Decomposition](02-backend-function-decomposition.md)** — High
   Break up `ws_logic.rs`, `archiver.rs`, and `messages.rs` into smaller functions.
   Replace `Arc<Mutex>` with `DashMap`. Fix nested control flow.

3. **[Testing Gaps](03-testing-gaps.md)** — High
   Add tests for the untested critical paths: ws_logic, archiver, nats consumer,
   frontend WebSocketService, and hooks.

4. **[Error Handling & Resilience](04-error-handling-resilience.md)** — Medium
   Remove startup panics, add auth caching, cap reconnection attempts,
   add timeouts to blocking operations.

5. **[Infrastructure Hardening](05-infrastructure-hardening.md)** — Low
   Docker persistent volumes, nginx gzip + TLS, resource limits, configurable CORS.

## Guiding Principles

All changes must follow the rules in `AGENTS.md`:
- Smallest change that works — do not bundle unrelated refactors
- Pure functions, immutable data, max 120 chars, max 15 lines per function
- Unit tests in separate files, written before fixtures
- Controller-service-repository layering (backend)
- Single global "database" state (frontend)
- Zero compilation warnings
