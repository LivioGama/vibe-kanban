# --- Chef stage: install cargo-chef ---
FROM node:24-alpine AS chef

RUN apk add --no-cache \
    curl \
    build-base \
    perl \
    llvm-dev \
    clang-dev

ENV RUSTFLAGS="-C target-feature=-crt-static"

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

RUN cargo install cargo-chef --locked

WORKDIR /app

# --- Planner stage: generate the dependency recipe ---
FROM chef AS planner

# Only copy Cargo manifests + stubs (no real source) so this layer
# is only invalidated when dependencies change, not source code
COPY Cargo.toml Cargo.lock ./
COPY crates/server/Cargo.toml ./crates/server/Cargo.toml
COPY crates/db/Cargo.toml ./crates/db/Cargo.toml
COPY crates/executors/Cargo.toml ./crates/executors/Cargo.toml
COPY crates/services/Cargo.toml ./crates/services/Cargo.toml
COPY crates/utils/Cargo.toml ./crates/utils/Cargo.toml
COPY crates/local-deployment/Cargo.toml ./crates/local-deployment/Cargo.toml
COPY crates/deployment/Cargo.toml ./crates/deployment/Cargo.toml
COPY crates/remote/Cargo.toml ./crates/remote/Cargo.toml
COPY crates/review/Cargo.toml ./crates/review/Cargo.toml

RUN mkdir -p crates/server/src/bin crates/db/src crates/executors/src \
    crates/services/src crates/utils/src crates/local-deployment/src \
    crates/deployment/src crates/remote/src/bin crates/review/src && \
    echo "fn main() {}" > crates/server/src/main.rs && \
    touch crates/server/src/lib.rs && \
    echo "fn main() {}" > crates/server/src/bin/generate_types.rs && \
    echo "fn main() {}" > crates/server/src/bin/mcp_task_server.rs && \
    touch crates/db/src/lib.rs && \
    touch crates/executors/src/lib.rs && \
    touch crates/services/src/lib.rs && \
    touch crates/utils/src/lib.rs && \
    touch crates/local-deployment/src/lib.rs && \
    touch crates/deployment/src/lib.rs && \
    echo "fn main() {}" > crates/remote/src/main.rs && \
    echo "fn main() {}" > crates/remote/src/bin/generate_types.rs && \
    echo "fn main() {}" > crates/review/src/main.rs

RUN cargo chef prepare --recipe-path recipe.json

# --- Builder stage ---
FROM chef AS builder

ARG POSTHOG_API_KEY
ARG POSTHOG_API_ENDPOINT

ENV VITE_PUBLIC_POSTHOG_KEY=$POSTHOG_API_KEY
ENV VITE_PUBLIC_POSTHOG_HOST=$POSTHOG_API_ENDPOINT
ENV SQLX_OFFLINE=true

# JS dependency caching
COPY package*.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY frontend/package*.json ./frontend/
COPY npx-cli/package*.json ./npx-cli/
RUN npm install -g pnpm && pnpm install

# Cook Rust dependencies (cached when Cargo.toml/Cargo.lock don't change)
COPY --from=planner /app/recipe.json recipe.json
COPY crates/db/.sqlx ./crates/db/.sqlx
COPY crates/remote/.sqlx ./crates/remote/.sqlx
RUN cargo chef cook --release --recipe-path recipe.json

# Copy source and build
COPY . .
RUN npm run generate-types
RUN cd frontend && pnpm run build
RUN cargo build --release --bin server

# --- Runtime stage ---
FROM alpine:latest AS runtime

RUN apk add --no-cache \
    ca-certificates \
    tini \
    libgcc \
    wget

RUN addgroup -g 1001 -S appgroup && \
    adduser -u 1001 -S -h /home/appuser -G appgroup appuser

COPY --from=builder /app/target/release/server /usr/local/bin/server

RUN mkdir -p /repos && \
    chown -R appuser:appgroup /repos

USER appuser

ENV HOME=/home/appuser
ENV HOST=0.0.0.0
ENV PORT=3000
EXPOSE 3000

WORKDIR /repos

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --quiet --tries=1 --spider "http://${HOST:-localhost}:${PORT:-3000}" || exit 1

ENTRYPOINT ["/sbin/tini", "--"]
CMD ["server"]
