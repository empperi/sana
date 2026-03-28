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
DATABASE_URL=postgres://sana_user:sana_password@127.0.0.1:5432/sana_db
NATS_URL=nats://127.0.0.1:4222
# Generate a 64-byte hex key for stable sessions: openssl rand -hex 64
COOKIE_KEY=000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f
```

**Note:** If `COOKIE_KEY` is not provided, the server will generate a new random key on every startup, which will invalidate all existing session cookies in your browser. Using a stable key in development prevents unexpected 401 Unauthorized errors when the backend restarts.

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

This project is optimized for agentic development with both **Gemini CLI** and **Claude Code**. Core operating principles, coding standards, and architecture guidelines are defined in [AGENTS.md](AGENTS.md) and loaded automatically by both tools.

- **[AGENTS.md](AGENTS.md)**: Shared operating principles, coding style, and architecture guidelines.
- **[GEMINI.md](GEMINI.md)**: Gemini-specific configuration (loaded automatically by Gemini CLI).
- **[CLAUDE.md](CLAUDE.md)**: Claude-specific configuration (loaded automatically by Claude Code).

### Activating Claude Code

Install Claude Code and start a session in the project root:

```bash
npm install -g @anthropic-ai/claude-code
claude
```

Claude Code will automatically load `CLAUDE.md` (which imports `AGENTS.md`) and connect to the configured MCP servers. Four domain-specific subagents are pre-configured in `.claude/agents/` and selected automatically based on the task at hand.

Install the Conductor plugin for structured multi-phase development workflows:

```
/plugin marketplace add lackeyjb/claude-conductor
/plugin install conductor@claude-conductor
/reload-plugins
```

### Activating Gemini CLI

Install Gemini CLI and start a session in the project root:

```bash
npm install -g @google/gemini-cli
gemini
```

Gemini CLI will automatically load `GEMINI.md` (which imports `AGENTS.md`) and connect to the configured MCP servers. Four domain-specific skills are pre-configured in `.gemini/skills/`.

Install the Conductor extension for structured multi-phase development workflows:
[gemini-cli-extensions/conductor](https://github.com/gemini-cli-extensions/conductor)

### Conductor — Structured Development Workflow

The `conductor/` directory contains implementation plans for significant features and architectural changes. Each plan is a markdown file describing the objective, background, affected files, implementation steps, and verification criteria.

Conductor provides a structured workflow for both tools:
- **Plan files** (`conductor/*.md`) define phases of work with clear steps and success criteria
- **Phased execution** — work through plans phase by phase with verification between each
- **Context-driven** — plans include all context an agent needs to implement safely without guessing


### Code Indexer (MCP)

Both tools use [probe by probelabs](https://github.com/probelabs/probe) for high-speed code search, symbol extraction, and AST-based structural queries with tree-sitter Rust support. It runs automatically via the respective tool's `settings.json` using a Docker container built from `Dockerfile.indexer` — no manual startup required.

Available MCP tools: `search_code`, `extract_code`, `grep`.

### Optional: cargo check hook

You can configure Claude Code to automatically run `cargo check` after every Rust file edit, surfacing compile errors back into the conversation immediately. This is configured in your personal, project-scoped `.claude/settings.local.json` (gitignored) so it only applies to this project and uses your own shell.

**Windows (PowerShell Core)** — create `.claude/settings.local.json`:

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "pwsh -NoProfile -File .claude/hooks/cargo-check.ps1"
          }
        ]
      }
    ]
  }
}
```

**Mac / Linux** — create `.claude/settings.local.json`:

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "sh .claude/hooks/cargo-check.sh"
          }
        ]
      }
    ]
  }
}
```

Both hook scripts are provided in `.claude/hooks/`. The Mac/Linux variant requires `python3` (pre-installed on modern macOS and most Linux distributions).

## Testing

Follow the TDD approach as specified in `AGENTS.md`.

- Run backend tests: `cargo test`
- Run frontend tests: `cargo test --manifest-path frontend/Cargo.toml`
