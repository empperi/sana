# ---- Build Stage ----
FROM rust:1-bookworm as builder

# Install Trunk and wasm32 target
RUN cargo install trunk
RUN rustup target add wasm32-unknown-unknown

# Create a new empty shell project
WORKDIR /usr/src/sana
RUN cargo init --bin .

# Copy over your manifests
COPY Cargo.toml Cargo.lock* ./

# Create a dummy frontend crate to satisfy the workspace
RUN mkdir -p frontend/src
COPY frontend/Cargo.toml ./frontend/Cargo.toml
RUN echo "fn main() {}" > frontend/src/main.rs

# Build only the dependencies to cache them
RUN cargo build --release

# Remove the dummy source code
RUN rm src/*.rs
RUN rm -rf frontend/src

# Copy your actual source code
COPY ./src ./src
COPY ./migrations ./migrations
COPY ./frontend ./frontend

# Build the backend
# We need to touch the main file to force a rebuild of the binary
RUN touch src/main.rs
RUN cargo build --release

# Build the frontend
WORKDIR /usr/src/sana/frontend
RUN trunk build --release

# ---- Runtime Stage ----
FROM debian:bookworm-slim as runtime

# Install OpenSSL and CA certificates which are often needed
RUN apt-get update && apt-get install -y libssl-dev ca-certificates curl && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from the build stage
COPY --from=builder /usr/src/sana/target/release/sana /usr/local/bin/sana

# Copy the frontend assets
# We need to place them where the backend expects them.
# The backend expects "frontend/dist" relative to the working directory.
WORKDIR /usr/local/bin
COPY --from=builder /usr/src/sana/frontend/dist ./frontend/dist

# Expose the port the app runs on
EXPOSE 3000

# Set the startup command to run the binary
CMD ["sana"]
