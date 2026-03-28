# Operating principles (non-negotiable)

1. **Smallest change that works**: Minimize blast radius; don't refactor adjacent code unless it meaningfully reduces
   risk or complexity.
2. **Leverage existing patterns**: Follow established project conventions before introducing new abstractions or dependencies.
3. **Prove it works**: "Seems right" is not done. Validate with tests/build/lint and/or a reliable manual repro.
4. **Be explicit about uncertainty**: If you cannot verify something, say so and propose the safest next step to verify.

# Coding style instructions

1. Use **only** pure functions unless Rust mechanics explicitly require otherwise OR if implementing side effect logic
   to write data to external source such as database, NATS, websocket or file system
2. Prefer immutable data over mutable unless it causes significant performance hit. If you do, ensure mutability is
   encapsulated in such a way that it cannot leak
3. Use TDD and implement tests for everything
4. With tests prefer unit tests over integration tests
5. With Rust unit tests, create those into separate test files, do not write them into actual implementation file as mod
6. With tests always write tests first in the file and fixtures and helper functions after the tests
7. Do not try to test private functions, use pure functions and functional style instead so testing public functions
   makes testing easy without excessive state setup
8. Keep lines at maximum of 120 characters unless it would make code less readable to split a line
9. On a rule of thumb, a function over 15 lines long is typically too long. Try to avoid longer functions unless
   explicitly unavoidable (for example, frontend HTML component structure definitions)
9. Any compilation warnings are explicitly forbidden, you **must** fix any warnings you encounter
10. Avoid unnecessary casting or .to_string() calls unless required by Rust compiler to make the code work
11. Nested code structures (if-statements, loops etc) should be avoided by using functions with return values instead

# Architecture guidelines (non-negotiable)

All architecture guidelines are non-negotiable and you must follow them unless explicitly told not to.

## General

- Strongly prefer acting on function return values instead of creating long call chains (command pattern)

## Backend

- Use controller-service-repository -layered architecture with similarly named source directories where
    - **controller**: handles REST and STOMP parsing and dispatching
    - **service**: contains all business logic, the only place where database transactions are opened or committed
    - **repository**: contains functions for calling database or NATS
- All inbound STOMP messages must be handled by pushing them with minimal logic to NATS and only when those messages
  come back should they be processed (read your own writes architecture)
- When implementing integration tests the test itself **must not** open database transactions, that is solely the
  responsibility of the service layer business logic

## Frontend

- All visible UI functionality **must** be encapsulated into components which are easy to move to different place
  in the UI
- All UI state **must** be in one global data structure called "database". Only exception is component local state,
  state that only makes sense in that component, example: input field suggestions
- All UI components **must** subscribe to the changes in the "database" and update themselves when data changes
- All business logic **must** be separated from the components unless it is explicitly business logic for the said
  component
- Only business logic is allowed to write to the "database", components may never directly do so
- Components communicate with business logic and other components via events and effects (re-frame way)

# Environment

This project runs on **Windows with PowerShell Core (`pwsh`)**. Use PowerShell syntax for all shell commands —
never bash or Unix shell syntax.

# MCP tools

- For symbol lookups (functions, structs, types, trait impls), high-level project mapping, and advanced searching,
  always prioritize the `code-indexer` MCP server over grep/glob tools.
- If the indexer is unavailable or fails, fall back to grep/glob/read tools.
- After large batches of changes, call the indexer's `reindex` tool to keep its context current.
- Whenever asked to "verify the UI", "check the page", or visually confirm a change, use the **Playwright** MCP
  server to navigate the app and confirm the result.

# Specialist agents / skills

Four domain specialists are available. Delegate to them proactively for their areas:

| Specialist | When to use |
|---|---|
| `rust-system-architect` | API endpoints, service/repository layer, SQLx queries, transactions, error handling, backend tests |
| `yew-frontend-specialist` | Yew components, custom hooks, global state, Wasm performance, frontend tests |
| `messaging-infra-specialist` | NATS consumers/subjects, STOMP frames, real-time delivery, reconnection logic |
| `visual-ux-designer` | SCSS styling, responsive layout, loading states, toast notifications, accessibility |
