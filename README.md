# Sana

Sana is a real-time messaging application featuring a Rust-based backend and a Yew WebAssembly frontend. It leverages NATS as a high-performance message broker to ensure reliable and scalable communication.

## Development Environment Setup

### Prerequisites

- **Rust**: Install via [rustup.rs](https://rustup.rs/).
- **WASM Target**: Add the WebAssembly target for Rust:
  ```bash
  rustup target add wasm32-unknown-unknown
  ```
- **Trunk**: The WASM web application bundler:
  ```bash
  cargo install trunk
  ```
- **Docker**: Used to run NATS locally.

### Infrastructure

The application requires a NATS server. You can start one using the provided Docker Compose configuration:

```bash
docker-compose up -d nats
```

### Running the Backend

The backend is an Axum server located in the root directory.

```bash
cargo run
```

By default, the server runs on `http://localhost:3000`.

### Running the Frontend (Watch Mode)

The frontend is a Yew application located in the `frontend` directory. To run it with hot-reloading:

```bash
cd frontend
trunk serve
```

Trunk will serve the frontend at `http://localhost:8080` (or another port if 8080 is occupied) and proxy requests to the backend if configured, or the backend will serve the assets from `frontend/dist`.

## Agentic Development

This project is optimized for agentic development. Please refer to the following files for guidance:

- **[GEMINI.md](GEMINI.md)**: Contains foundational mandates and workspace-specific instructions for AI agents.
- **[AGENTS.md](AGENTS.md)**: Defines the core operating principles and coding style instructions that must be followed.

## Testing

Follow the TDD approach as specified in `AGENTS.md`.

- Run backend tests: `cargo test`
- Run frontend tests: `cd frontend && cargo test`
