# Sana

Sana is a real-time messaging application featuring a Rust-based backend and a Yew WebAssembly frontend. It leverages NATS as a high-performance message broker to ensure reliable and scalable communication and PostgreSQL for persistent data storage.

## Development Environment Setup

### Prerequisites

- **Rust**: Install via [rustup.rs](https://rustup.rs/).
- **WASM Target**: Add the WebAssembly target for Rust:
  ```bash
  rustup target add wasm32-unknown-unknown
  ```
- **Trunk**: The WASM web application bundler used for the frontend:
  ```bash
  cargo install trunk
  ```
- **sqlx-cli**: While migrations run automatically on startup, this CLI is highly recommended for creating new migrations and managing the database schema during development:
  ```bash
  cargo install sqlx-cli --no-default-features --features rustls,postgres
  ```
- **Docker & Docker Compose**: Used to run NATS and PostgreSQL locally.

### Infrastructure

The application requires both a NATS server (with JetStream enabled) and a PostgreSQL database. You can start them using the provided Docker Compose configuration:

```bash
docker-compose up -d nats db
```

*Note: The `db` service automatically provisions a user `sana_user`, password `sana_password`, and database `sana_db` exposed on port `5432`.*

### Environment Configuration

The backend uses a combination of `.env` files and `config.json` for configuration. Environment variables take precedence over the JSON file. 

Create a `.env` file in the root directory for your local database and NATS connection:

```env
DATABASE_URL=postgres://sana_user:sana_password@localhost:5432/sana_db
NATS_URL=nats://localhost:4222
```

### Running the Backend

The backend is an Axum server located in the root directory. It runs database migrations automatically on startup.

```bash
cargo run
```

By default, the server runs on `http://localhost:3000`.

### Running the Frontend (Watch Mode)

The frontend is a Yew application located in the `frontend` directory. It uses Trunk to proxy API requests and WebSocket connections to the local backend on port `3000`. 

To run the frontend with hot-reloading:

```bash
cd frontend
trunk serve
```

Trunk will serve the frontend at `http://localhost:8080`. Navigating to `http://localhost:8080` will display the web interface and interact seamlessly with the running backend.

### Full Distributed Stack (Docker)

To run the complete system with a load balancer, multiple backend replicas, NATS, and PostgreSQL:

```bash
docker-compose up --build -d
```

Access the application at `http://localhost:8080`. The load balancer (NGINX) will distribute traffic across backend instances.

## Agentic Development

This project is optimized for agentic development. Please refer to the following files for guidance:

- **[GEMINI.md](GEMINI.md)**: Contains foundational mandates and workspace-specific instructions for AI agents.
- **[AGENTS.md](AGENTS.md)**: Defines the core operating principles and coding style instructions that must be followed.

## Testing

Follow the TDD approach as specified in `AGENTS.md`.

- Run backend tests: `cargo test`
- Run frontend tests: `cargo test --manifest-path frontend/Cargo.toml`
