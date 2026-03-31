# Implementation Plan: End-to-end Tests

## Phase 1: Environment Setup & Infrastructure [checkpoint: dfe509c]
Setup the foundations for E2E testing, including directory structure and Docker orchestration.

- [x] Task: Update `tech-stack.md` to document Playwright and Node.js as E2E testing dependencies (required by workflow before introducing new tech).
- [x] Task: Create E2E directory structure and initialize Playwright project.
    - [x] Create top-level `e2e/` directory (not under `tests/` which contains Rust integration tests).
    - [x] Initialize Node.js/Playwright project within `e2e/`.
    - [x] Configure `playwright.config.ts` with base URL (`http://localhost:8080`) and browser settings.
- [x] Task: Configure Docker Compose for E2E testing.
    - [x] Create `docker-compose.e2e.yml` (separate from dev compose, use `--project-name e2e` to avoid conflicts).
    - [x] Use 2 app replicas behind NGINX (matching production) to validate the stateless multi-instance contract.
    - [x] Use an ephemeral (unnamed) PostgreSQL volume so every run starts with a clean database.
    - [x] Generate a `COOKIE_KEY` and inject it as an environment variable before starting the stack.
    - [x] Add a migration runner step (e.g. `sqlx migrate run` via init container or startup script) before the app serves traffic.
    - [x] Start with `docker compose --project-name e2e up --wait` so the CI step blocks until all healthchecks pass.
- [x] Task: Implement basic health-check test.
    - [x] Write a test that navigates to the app and confirms the landing page loads.
- [ ] Task: Conductor - User Manual Verification 'Phase 1: Environment Setup & Infrastructure' (Protocol in workflow.md)

## Phase 2: Authentication Flows
Automate the core identity and session management flows.

- [x] Task: Implement User Registration test.
    - [x] Automate the signup form submission.
    - [x] Verify successful registration and redirection.
- [x] Task: Implement User Login/Logout test.
    - [x] Automate the login process.
    - [x] Verify session persistence across page reloads.
    - [x] Automate the logout process and verify session termination.
- [x] Task: Conductor - User Manual Verification 'Phase 2: Authentication Flows' (Protocol in workflow.md)

## Phase 3: Messaging and Channel Interactions [checkpoint: 153dd34]
Automate real-time communication flows involving multi-user interactions.

- [x] Task: Implement Channel Management tests.
    - [x] Verify user can view public channels.
    - [x] Verify user can join a public channel.
- [x] Task: Implement Real-time Messaging test.
    - [x] Setup multi-browser context in Playwright (2 separate browser sessions, each authenticated as a different user).
    - [x] Verify user A can send a message and user B receives it in real-time.
    - [x] Each test that writes data must clean up after itself or use uniquely-scoped test data (e.g. unique usernames per test run) to ensure isolation.
- [x] Task: Conductor - User Manual Verification 'Phase 3: Messaging and Channel Interactions' (Protocol in workflow.md)

## Phase 4: CI Integration & Finalization [checkpoint: 46738a3]
Integrate the suite into the automation pipeline and finalize documentation.

- [x] Task: Integrate E2E suite into GitHub Actions.
    - [x] Add a new `e2e` job in `.github/workflows/ci.yml` (separate from the existing `test` job; different requirements: Docker Compose, Node.js).
    - [x] Include steps: checkout, generate `COOKIE_KEY`, `docker compose --project-name e2e up --wait`, run Playwright, `docker compose down`.
    - [x] Configure cache for Node.js dependencies and Playwright browsers.
    - [x] Always run `docker compose --project-name e2e down` in a `if: always()` step to avoid leaked containers on failure.
- [x] Task: Implement Artifact Capture.
    - [x] Configure Playwright to capture screenshots/videos/traces on test failure.
    - [x] Ensure artifacts are uploaded to GitHub Actions.
- [x] Task: Finalize Documentation.
    - [x] Add instructions to `README.md` or a dedicated doc for running E2E tests locally.
- [ ] Task: Conductor - User Manual Verification 'Phase 4: CI Integration & Finalization' (Protocol in workflow.md)
