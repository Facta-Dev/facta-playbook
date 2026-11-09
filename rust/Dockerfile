# Multi-stage Dockerfile for `<app>-api`.
# ponytail: rename every `<app>` -> your project name before building.
# The `<app>` placeholder is NOT a valid crate name, so this image will not
# build until the rename has been applied (see README.md "How to apply").
#
# Build:
#   docker build -t <app>-api .
# Run (see docker-compose.yml for the full stack):
#   docker run --rm -p 8080:8080 --env-file .env <app>-api
#
# The worker binary (/app/worker, `<app>-worker`) shares this image; compose
# overrides the entrypoint per service.

# ───────────────────────── builder ─────────────────────────
# Rolling 1.x base (>= MSRV 1.88). rust-toolchain.toml is COPYed below and pins
# the exact 1.93.1 channel, so the base only needs to be >= MSRV — the old
# `rust:1.85-bookworm` pin was STALE (below MSRV) and is removed.
FROM rust:1-bookworm AS builder

# go/py parity: all sqlx queries are RUNTIME queries (no compile-time macros,
# no .sqlx cache) — the build needs no database at all, and the worker uses the
# same pool at runtime.
WORKDIR /build

# Cache deps: copy manifests first, then dummy-build them so the layer is
# reused across source-only changes.
COPY Cargo.toml deny.toml rust-toolchain.toml ./
COPY crates/ ./crates/
COPY migrations/ ./migrations/

# Build both release binaries: the HTTP API server and the outbox-email worker
# (register/forgot-password deliveries + expired-session janitor).
RUN cargo build --release --bin <app>-api --bin <app>-worker

# ───────────────────────── runtime ─────────────────────────
# distroless cc-debian12 ships glibc + ca-certificates + a nonroot user
# (UID/GID 65532) and nothing else: no shell, no package manager, no attack
# surface. Rust release binaries link glibc dynamically (libc, libgcc), so the
# cc (not base) variant is required.
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime

# Nonroot UID/GID 65532 — matches the distroless :nonroot image. Stated
# explicitly so the hardening is visible and survives a base-image swap.
USER 65532:65532

WORKDIR /app

# Copy both binaries and the migrations so the runtime image is
# self-contained. The migration files are informational (the API does not run
# them at startup); keep them for a `sqlx migrate run` sidecar if adopted.
COPY --from=builder /build/target/release/<app>-api /app/server
COPY --from=builder /build/target/release/<app>-worker /app/worker
COPY --from=builder /build/migrations/ /app/migrations/

# Expose the HTTP listener. <APP>_BIND defaults to 0.0.0.0:8080 (see .env.example).
EXPOSE 8080

# No shell in distroless — invoke the binary directly.
ENTRYPOINT ["/app/server"]