# ============================================
# Stage 1: Prepare Rust dependencies
# ============================================
FROM lukemathwalker/cargo-chef:latest-rust-1.90 AS chef
WORKDIR /app

FROM chef AS planner
COPY backend/ .
RUN cargo chef prepare --recipe-path recipe.json

# ============================================
# Stage 2: Build Rust binaries and generate fixtures
# ============================================
FROM chef AS rust-builder

# Build dependencies first (cached layer)
COPY --from=planner /app/recipe.json recipe.json
# `snmp2` is a path dependency (vendor/snmp2). cargo-chef's cook step compiles
# dependencies from recipe.json, so the path-dep source must be present here or
# cook fails reading vendor/snmp2/Cargo.toml. Copy it before cooking (it rarely
# changes, so the dependency cache layer stays warm).
COPY backend/vendor/ ./vendor/
RUN cargo chef cook --release --recipe-path recipe.json

# Build server and generate-fixtures binaries
COPY backend/ .

# Download fresh OUI vendor database before compiling
RUN curl -sf --max-time 60 -o assets/oui.csv https://standards-oui.ieee.org/oui/oui.csv || echo "Using bundled OUI database"

RUN cargo build --release --bin server --bin generate-fixtures

# Verify binary exists
RUN test -f /app/target/release/server || (echo "Server binary not found" && exit 1)

# Generate fixture JSON files for the UI build
RUN /app/target/release/generate-fixtures --output-dir /fixtures

# ============================================
# Stage 3: Build UI (SvelteKit static site)
# ============================================
FROM node:20-slim AS ui-builder

WORKDIR /app/ui

# Copy package files and install dependencies
COPY ui/package*.json ./
RUN npm ci

# Copy UI source code
COPY ui/ ./

# Copy i18n messages (paraglide source - referenced by ../messages/ from project.inlang)
COPY messages/ ../messages/

# Copy generated fixtures from rust-builder
COPY --from=rust-builder /fixtures/ ./src/lib/data/

# Copy downloaded service logos from rust-builder
COPY --from=rust-builder /static/logos/services/ ./static/logos/services/

# Accept build-time configuration
ARG PUBLIC_SERVER_HOSTNAME=default
ARG PUBLIC_SERVER_PORT=60072

# Make available during build
ENV PUBLIC_SERVER_HOSTNAME=$PUBLIC_SERVER_HOSTNAME
ENV PUBLIC_SERVER_PORT=$PUBLIC_SERVER_PORT

# Build static UI assets
RUN npm run build

# Verify build output exists
RUN ls -la /app/ui/build && \
    test -f /app/ui/build/index.html || (echo "Build failed: no index.html found" && exit 1)

# ============================================
# Stage 4: Final runtime image
# ============================================
FROM debian:bookworm-slim AS runtime

# Required by Kamal for container management 
LABEL service="scanopy"

# OCI Image Spec labels — https://github.com/opencontainers/image-spec/blob/main/annotations.md
ARG OCI_IMAGE_TITLE="Scanopy Server"
ARG OCI_IMAGE_DESCRIPTION="Scanopy server — Network diagrams that update themselves."
ARG OCI_IMAGE_LICENSES="AGPL-3.0-only"
LABEL org.opencontainers.image.title="${OCI_IMAGE_TITLE}" \
      org.opencontainers.image.description="${OCI_IMAGE_DESCRIPTION}" \
      org.opencontainers.image.url="https://github.com/scanopy/scanopy" \
      org.opencontainers.image.source="https://github.com/scanopy/scanopy" \
      org.opencontainers.image.documentation="https://github.com/scanopy/scanopy#readme" \
      org.opencontainers.image.vendor="Scanopy" \
      org.opencontainers.image.licenses="${OCI_IMAGE_LICENSES}"

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy server binary from rust-builder stage
COPY --from=rust-builder /app/target/release/server /usr/local/bin/server
RUN chmod +x /usr/local/bin/server

# Migrations are compiled into the binary (rust-embed, see migration_runner.rs),
# so there is nothing to copy here and the server no longer depends on its
# working directory to find them.

# Copy UI static assets from ui-builder stage
COPY --from=ui-builder /app/ui/build ./static

# Verify UI files are present
RUN ls -la /app/static && \
    test -f /app/static/index.html || (echo "UI files not copied correctly" && exit 1)

EXPOSE 60072

# Health check
HEALTHCHECK --interval=10s --timeout=5s --retries=5 \
    CMD curl -f http://localhost:60072/api/health || exit 1

# Run the server
CMD ["server"]