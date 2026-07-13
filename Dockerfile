# syntax=docker/dockerfile:1

# ---- Build Stage ----
FROM rust:1-bookworm as builder

# Install the wasm32 target
RUN rustup target add wasm32-unknown-unknown

# Install Trunk from a prebuilt release binary. This avoids `cargo install trunk`,
# which compiles it from source (~minutes) on every cold build. Pinned for
# reproducibility; bump TRUNK_VERSION to upgrade. `uname -m` yields x86_64 /
# aarch64, which matches Trunk's release asset naming, so this builds on both
# amd64 and arm64 hosts.
ARG TRUNK_VERSION=0.21.14
RUN arch="$(uname -m)" \
    && curl -fsSL "https://github.com/trunk-rs/trunk/releases/download/v${TRUNK_VERSION}/trunk-${arch}-unknown-linux-gnu.tar.gz" \
    | tar -xzf - -C /usr/local/bin trunk

WORKDIR /usr/src/sana

# Copy manifests and source. Dependency artifacts are cached via the BuildKit
# cache mounts below, so the old "dummy crate" pre-build trick is no longer needed.
COPY Cargo.toml Cargo.lock* ./
COPY frontend/Cargo.toml ./frontend/Cargo.toml
COPY ./src ./src
COPY ./migrations ./migrations
COPY ./frontend ./frontend

# Build the backend.
# The cargo registry, git cache, and target dir persist across builds, so
# dependencies are only recompiled when their versions actually change.
# The target dir is a cache mount (not part of the image layer), so the binary
# must be copied out to a normal path within the same RUN for the runtime stage.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/usr/src/sana/target,sharing=locked \
    cargo build --release && cp target/release/sana /usr/local/bin/sana

# Build the frontend. Reuses the same cached target dir, so the wasm dependency
# tree is no longer recompiled on every build. `frontend/dist` is outside the
# target dir, so it persists into the image layer for the COPY below.
WORKDIR /usr/src/sana/frontend
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/usr/src/sana/target,sharing=locked \
    trunk build --release

# ---- Runtime Stage ----
FROM debian:bookworm-slim as runtime

# Install OpenSSL and CA certificates which are often needed
RUN apt-get update && apt-get install -y libssl-dev ca-certificates curl && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from the build stage
COPY --from=builder /usr/local/bin/sana /usr/local/bin/sana

# Copy the frontend assets
# We need to place them where the backend expects them.
# The backend expects "frontend/dist" relative to the working directory.
WORKDIR /usr/local/bin
COPY --from=builder /usr/src/sana/frontend/dist ./frontend/dist

# Expose the port the app runs on
EXPOSE 3000

# Set the startup command to run the binary
CMD ["sana"]
