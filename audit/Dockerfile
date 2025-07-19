FROM rust:1.88-alpine3.22 AS builder

RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    openssl-libs-static \
    pkgconf \
    libpq-dev

COPY audit/Cargo.toml /app/audit/Cargo.toml
COPY audit/Cargo.lock /app/audit/Cargo.lock

# -- caching dependencies
# Create a dummy main.rs for cargo to build --something--, 
# build the application, then remove the dummy main.rs
# and the final cached binary (so cargo build will not reuse the cached binary) 
RUN mkdir -p /app/audit/src && \
    echo 'fn main() { println!("Hello, world!"); }' > /app/audit/src/main.rs && \
    cd /app/audit && \
    cargo build --release --target x86_64-unknown-linux-musl && \
    cargo clean --target x86_64-unknown-linux-musl -p audit && \
    rm -rf /app/audit/src && \
    rm -rf /app/audit/target/x86_64-unknown-linux-musl/release/deps/audit*

# -- end caching dependencies


COPY audit/src /app/audit/src

WORKDIR /app/audit

RUN touch /app/audit/src/main.rs && cargo build --release --target x86_64-unknown-linux-musl


FROM alpine:3.22

COPY --from=builder /app/audit/target/x86_64-unknown-linux-musl/release/audit /usr/local/bin/audit

# Create a non-root user without login priviledges to reduce the potential attack surface somewhat
RUN addgroup -g 1000 audituser && \
    adduser -D -u 1000 -G audituser audituser && \
    chown audituser:audituser /usr/local/bin/audit

USER audituser

ENTRYPOINT [ "audit" ]
