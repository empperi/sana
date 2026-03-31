# Specification: End-to-end Tests

## Overview
Implement a comprehensive end-to-end (E2E) testing suite for the Sana application to ensure critical user flows remain functional as the project evolves. This suite will supplement existing unit and integration tests, focusing on the integration of the Yew frontend with the Rust backend via NATS and PostgreSQL.

## Functional Requirements
- **Testing Framework:** Utilize **Playwright** (JS-based) for orchestrating browser-level interactions.
- **Environment Orchestration:** Use a dedicated `docker-compose.e2e.yml` to spin up the full Sana stack (NGINX, Backend, Frontend, NATS, PostgreSQL) for testing.
  - The stack must use 2 app replicas (matching production) so that tests validate the stateless/session-agnostic contract across instances.
  - The database must be ephemeral (no named volume) so every test run starts with a clean state.
  - A `COOKIE_KEY` secret must be generated and injected into the environment before tests run.
  - SQLx database migrations must be applied automatically before the app starts accepting traffic.
- **Core User Flows:**
    - **Authentication:** Verify user registration, login, and session persistence.
    - **Messaging:** Verify real-time message sending and delivery across multiple browser sessions.
    - **Channel Management:** Verify joining public channels and updating the channel list.
- **CI Integration:** Integrate the E2E suite into a dedicated `e2e` GitHub Actions job (separate from the existing `test` job), running on every pull request to `main`.

## Non-Functional Requirements
- **Test Reliability:** Implement robust waiting and synchronization logic (e.g., waiting for WebSocket connections or DOM elements) to minimize test flakiness.
- **Test Performance:** Ensure the E2E suite executes within a reasonable time frame (target: < 5 minutes for core flows).
- **Reportability:** Generate human-readable test reports and capture screenshots/videos on failure.

## Acceptance Criteria
- [ ] A dedicated `e2e` directory is created for Playwright tests.
- [ ] Docker Compose configuration is updated or created to support a consistent test environment.
- [ ] Automated tests cover the Auth, Messaging, and Channel Management flows.
- [ ] GitHub Actions workflow is successfully updated to run E2E tests on PRs.
- [ ] Documentation provided for running E2E tests locally.

## Out of Scope
- **Performance/Load Testing:** High-volume traffic simulation is not part of this track.
- **File Sharing:** E2E validation for file uploads and image rendering is deferred.
- **Mobile Device Support:** Initial testing will focus on desktop browser views.
