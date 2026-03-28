# Technology Stack

## Core Language
- **Rust:** The primary programming language for both the backend and frontend, chosen for its performance, safety, and modern toolchain.

## Backend Architecture
- **Axum:** The web framework used for building the RESTful and WebSocket API endpoints.
- **SQLx:** The database driver and toolkit for interacting with PostgreSQL, providing compile-time query verification and migrations.
- **async-nats:** The client library for interacting with NATS JetStream for high-performance messaging.
- **Tokio:** The underlying asynchronous runtime for high-throughput and low-latency networking.

## Frontend Architecture
- **Yew:** The WebAssembly framework used for building a fast and responsive user interface in Rust.
    - **State Management:** Single global "database" state using **Yew Context** and the **Reducible** pattern for predictable state mutations.
- **Trunk:** The web application bundler and development server used to proxy API requests and build the Wasm binary.
- **Gloo:** A collection of high-level Rust wrappers for common browser APIs (HTTP, timers, events).

## Data & Messaging
- **PostgreSQL:** The primary database for persistent data storage, including user profiles and message history.
- **NATS (JetStream):** The high-performance message broker used for real-time communication and reliable message delivery.

## Infrastructure & Deployment
- **Development Infrastructure:**
    - **Docker & Docker Compose:** Used for local development and for orchestrating the application's services (NATS, Postgres, Axum, NGINX).
- **Production Infrastructure (Planned):**
    - **Kubernetes:** The target platform for production deployment, orchestration, and scaling.
- **NGINX:** The reverse proxy and load balancer used to route traffic and serve static assets.
