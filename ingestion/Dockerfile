FROM rust:1.88-alpine3.22 AS builder

RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    openssl-libs-static \
    pkgconf \
    libpq-dev

COPY libs/auth-check /app/libs/auth-check
COPY libs/audit /app/libs/audit

COPY ingestion/Cargo.toml /app/ingestion/Cargo.toml
COPY ingestion/Cargo.lock /app/ingestion/Cargo.lock

# -- caching dependencies
# Create a dummy main.rs for cargo to build --something--, 
# build the application, then remove the dummy main.rs
# and the final cached binary (so cargo build will not reuse the cached binary) 
RUN mkdir -p /app/ingestion/src && \
    echo 'fn main() { println!("Hello, world!"); }' > /app/ingestion/src/main.rs && \
    cd /app/ingestion && \
    cargo build --release --target x86_64-unknown-linux-musl && \
    cargo clean --target x86_64-unknown-linux-musl -p ingestion && \
    rm -rf /app/ingestion/src && \
    rm -rf /app/ingestion/target/x86_64-unknown-linux-musl/release/deps/ingestion*

# -- end caching dependencies


COPY ingestion/src /app/ingestion/src

WORKDIR /app/ingestion

RUN touch /app/ingestion/src/main.rs && cargo build --release --target x86_64-unknown-linux-musl


FROM alpine:3.22

COPY --from=builder /app/ingestion/target/x86_64-unknown-linux-musl/release/ingestion /usr/local/bin/ingestion

# Create a non-root user without login priviledges to reduce the potential attack surface somewhat
RUN addgroup -g 1000 ingestionuser && \
    adduser -D -u 1000 -G ingestionuser ingestionuser && \
    chown ingestionuser:ingestionuser /usr/local/bin/ingestion

USER ingestionuser

ENTRYPOINT [ "ingestion" ]
