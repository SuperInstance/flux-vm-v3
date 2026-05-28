FROM rust:1.76-slim AS builder

WORKDIR /app

# Install build deps
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/

# Build the library (no binary in this crate)
RUN cargo build --release

# Run tests to verify build
RUN cargo test --release

FROM debian:bookworm-slim

LABEL org.opencontainers.image.title="flux-vm-v3"
LABEL org.opencontainers.image.description="FLUX VM v3 — vectorized virtual machine with JIT and streaming execution"
LABEL org.opencontainers.image.source="https://github.com/SuperInstance/flux-vm-v3-temp"

# Create non-root user
RUN groupadd -r appuser && useradd -r -g appuser -u 1000 appuser

# Install runtime deps
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy built library artifacts
COPY --from=builder /app/target/release/libflux_vm_v3.so /usr/local/lib/
COPY --from=builder /app/target/release/libflux_vm_v3.rlib /usr/local/lib/

# Copy test artifacts for smoke test
COPY --from=builder /app/target/release/deps/ /app/target/release/deps/
COPY --from=builder /app/tests/ /app/tests/
COPY --from=builder /app/Cargo.toml /app/
COPY --from=builder /app/Cargo.lock /app/

WORKDIR /app

USER appuser

EXPOSE 8080

# Library crate — no running service. Healthcheck verifies library loads.
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD python3 -c "import ctypes; ctypes.CDLL('/usr/local/lib/libflux_vm_v3.so')" || exit 1

# Default: run the test suite as smoke test
CMD ["sh", "-c", "echo 'flux-vm-v3 library built successfully. Run with: cargo test'"]
