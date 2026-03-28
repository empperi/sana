# Phase 5: Infrastructure Hardening

## Objective
Improve the Docker and nginx configuration for production readiness. These are lower priority
than code changes but important before any real deployment.

## Issues & Fixes

### 5a. PostgreSQL data persistence

**Problem:** `docker-compose.yml` does not define a persistent volume for the `db` service.
On `docker-compose down`, all data is lost.

**Fix:** Add a named volume:
```yaml
db:
  volumes:
    - pgdata:/var/lib/postgresql/data

volumes:
  pgdata:
```

### 5b. Nginx — no gzip compression

**Problem:** `nginx.conf` serves all responses uncompressed. WASM binaries and JSON payloads
benefit significantly from compression.

**Fix:** Add gzip block:
```nginx
gzip on;
gzip_types text/plain text/css application/json application/javascript application/wasm;
gzip_min_length 1000;
```

### 5c. Nginx — no cache headers for static assets

**Problem:** Frontend dist files (JS, WASM, CSS) are served without `Cache-Control` headers.
Browsers re-fetch on every page load.

**Fix:** Add location block for static assets:
```nginx
location ~* \.(js|wasm|css|png|svg|ico)$ {
    expires 1y;
    add_header Cache-Control "public, immutable";
}
```

### 5d. CORS hardcoded to localhost

**Problem:** `src/router.rs` line 13 hardcodes CORS origin to `localhost:8080`. This breaks
any non-local deployment.

**Fix:** Make CORS origin configurable via `config.rs`:
```rust
let cors_origin = config.get_value("cors_origin")
    .unwrap_or_else(|| "http://localhost:8080".to_string());
```

### 5e. No resource limits in Docker Compose

**Problem:** No memory or CPU limits on any service. A memory leak or runaway process can
consume all host resources.

**Fix:** Add deploy resource limits:
```yaml
app:
  deploy:
    replicas: 2
    resources:
      limits:
        memory: 512M
        cpus: '1.0'

nats:
  deploy:
    resources:
      limits:
        memory: 256M

db:
  deploy:
    resources:
      limits:
        memory: 512M
```

### 5f. COOKIE_KEY hardcoded in docker-compose.yml

**Problem:** The cookie signing key is committed in plaintext in `docker-compose.yml`.

**Fix:** Reference from `.env` file:
```yaml
environment:
  COOKIE_KEY: ${COOKIE_KEY}
```

And document that `.env` must contain a `COOKIE_KEY` value for the Docker stack.

### 5g. No health checks for app service

**Problem:** Docker Compose has a health check for `db` but not for the `app` service.
The load balancer may route to an app instance that hasn't finished starting.

**Fix:** Add health check endpoint to backend (`GET /health`) and configure in docker-compose:
```yaml
app:
  healthcheck:
    test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
    interval: 10s
    timeout: 5s
    retries: 3
```

## Implementation Steps

1. Add pgdata volume to docker-compose.yml
2. Add gzip and cache headers to nginx.conf
3. Make CORS origin configurable in config.rs and router.rs
4. Add resource limits to docker-compose.yml
5. Move COOKIE_KEY to .env reference
6. Add /health endpoint and health check to docker-compose.yml

## Verification
- `docker-compose up --build -d` — all services start and pass health checks
- `curl -I localhost:8080/app.wasm` — verify gzip and cache headers present
- `docker-compose down && docker-compose up -d` — verify pgdata survives restart
- Test with different CORS origin — verify configurable

## Risk
These changes are low risk — infrastructure configuration only, no application logic changes.
Each can be deployed independently.
