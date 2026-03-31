# Implementation Plan: End-to-end Tests

## Phase 1: Environment Setup & Infrastructure
Setup the foundations for E2E testing, including directory structure and Docker orchestration.

- [ ] Task: Update `tech-stack.md` to document Playwright and Node.js as E2E testing dependencies (required by workflow before introducing new tech).
- [ ] Task: Create E2E directory structure and initialize Playwright project.
    - [ ] Create top-level `e2e/` directory (not under `tests/` which contains Rust integration tests).
    - [ ] Initialize Node.js/Playwright project within `e2e/`.
    - [ ] Configure `playwright.config.ts` with base URL (`http://localhost:8080`) and browser settings.
- [ ] Task: Configure Docker Compose for E2E testing.
    - [ ] Create `docker-compose.e2e.yml` (separate from dev compose, use `--project-name e2e` to avoid conflicts).
    - [ ] Use 2 app replicas behind NGINX (matching production) to validate the stateless multi-instance contract.
    - [ ] Use an ephemeral (unnamed) PostgreSQL volume so every run starts with a clean database.
    - [ ] Generate a `COOKIE_KEY` and inject it as an environment variable before starting the stack.
    - [ ] Add a migration runner step (e.g. `sqlx migrate run` via init container or startup script) before the app serves traffic.
    - [ ] Start with `docker compose --project-name e2e up --wait` so the CI step blocks until all healthchecks pass.
- [ ] Task: Implement basic health-check test.
    - [ ] Write a test that navigates to the app and confirms the landing page loads.
- [ ] Task: Conductor - User Manual Verification 'Phase 1: Environment Setup & Infrastructure' (Protocol in workflow.md)

## Phase 2: Authentication Flows
Automate the core identity and session management flows.

- [ ] Task: Implement User Registration test.
    - [ ] Automate the signup form submission.
    - [ ] Verify successful registration and redirection.
- [ ] Task: Implement User Login/Logout test.
    - [ ] Automate the login process.
    - [ ] Verify session persistence across page reloads.
    - [ ] Automate the logout process and verify session termination.
- [ ] Task: Conductor - User Manual Verification 'Phase 2: Authentication Flows' (Protocol in workflow.md)

## Phase 3: Messaging and Channel Interactions
Automate real-time communication flows involving multi-user interactions.

- [ ] Task: Implement Channel Management tests.
    - [ ] Verify user can view public channels.
    - [ ] Verify user can join a public channel.
- [ ] Task: Implement Real-time Messaging test.
    - [ ] Setup multi-browser context in Playwright (2 separate browser sessions, each authenticated as a different user).
    - [ ] Verify user A can send a message and user B receives it in real-time.
    - [ ] Each test that writes data must clean up after itself or use uniquely-scoped test data (e.g. unique usernames per test run) to ensure isolation.
- [ ] Task: Conductor - User Manual Verification 'Phase 3: Messaging and Channel Interactions' (Protocol in workflow.md)

## Phase 4: CI Integration & Finalization
Integrate the suite into the automation pipeline and finalize documentation.

- [ ] Task: Integrate E2E suite into GitHub Actions.
    - [ ] Add a new `e2e` job in `.github/workflows/ci.yml` (separate from the existing `test` job; different requirements: Docker Compose, Node.js).
    - [ ] Include steps: checkout, generate `COOKIE_KEY`, `docker compose --project-name e2e up --wait`, run Playwright, `docker compose down`.
    - [ ] Configure cache for Node.js dependencies and Playwright browsers.
    - [ ] Always run `docker compose --project-name e2e down` in a `if: always()` step to avoid leaked containers on failure.
- [ ] Task: Implement Artifact Capture.
    - [ ] Configure Playwright to capture screenshots/videos/traces on test failure.
    - [ ] Ensure artifacts are uploaded to GitHub Actions.
- [ ] Task: Finalize Documentation.
    - [ ] Add instructions to `README.md` or a dedicated doc for running E2E tests locally.
- [ ] Task: Conductor - User Manual Verification 'Phase 4: CI Integration & Finalization' (Protocol in workflow.md)
