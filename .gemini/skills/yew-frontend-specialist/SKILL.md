---
name: yew-frontend-specialist
description: Expert in Yew WebAssembly development, custom hooks, and state management for the Sana project. Use when creating new UI components, refactoring frontend logic into hooks, or optimizing Wasm performance.
---

# Yew Frontend Specialist

This skill focuses on the frontend architecture of Sana using Yew.

## Hook-First Architecture
- **Isolate Side-Effects**: Move all API calls, WebSocket subscriptions, and DOM event listeners into custom hooks.
- **State Management**: Use `use_state` for local component state and `use_context` for shared application state (e.g., user session).

## Component Design
- **Functional Style**: Exclusively use functional components with the `#[function_component]` macro.
- **Prop Drilling**: Avoid deep nesting; use contexts or extract sub-components to keep props manageable.

## Performance
- **Avoid Heavy Compute in Render**: Use `use_memo` for expensive calculations.
- **Wasm Optimization**: Be mindful of large clones during render cycles. Use `AttrValue` for strings where appropriate.

## Testing
- **Unit Test Logic**: Move non-UI logic to `logic.rs` and test it with standard Rust unit tests.
