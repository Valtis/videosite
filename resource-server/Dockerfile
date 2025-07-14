FROM rust:1.88-alpine3.22 AS builder

RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    openssl-libs-static \
    pkgconf \
    libpq-dev

COPY libs/auth-check /app/libs/auth-check

COPY resource-server/Cargo.toml /app/resource-server/Cargo.toml
COPY resource-server/Cargo.lock /app/resource-server/Cargo.lock

# -- caching dependencies
# Create a dummy main.rs for cargo to build --something--, 
# build the application, then remove the dummy main.rs
# and the final cached binary (so cargo build will not reuse the cached binary) 
RUN mkdir -p /app/resource-server/src && \
    echo 'fn main() { println!("Hello, world!"); }' > /app/resource-server/src/main.rs && \
    cd /app/resource-server && \
    cargo build --release --target x86_64-unknown-linux-musl && \
    cargo clean --target x86_64-unknown-linux-musl -p resource-server && \
    rm -rf /app/resource-server/src && \
    rm -rf /app/resource-server/target/x86_64-unknown-linux-musl/release/deps/resource-server*

# -- end caching dependencies


COPY resource-server/src /app/resource-server/src

WORKDIR /app/resource-server

RUN ls /app/resource-server/target/x86_64-unknown-linux-musl/release -la
RUN touch /app/resource-server/src/main.rs && cargo build --release --target x86_64-unknown-linux-musl


FROM alpine:3.22

COPY --from=builder /app/resource-server/target/x86_64-unknown-linux-musl/release/resource-server /usr/local/bin/resource-server

# Create a non-root user without login priviledges to reduce the potential attack surface somewhat
RUN addgroup -g 1000 resourceuser && \
    adduser -D -u 1000 -G resourceuser resourceuser && \
    chown resourceuser:resourceuser /usr/local/bin/resource-server

USER resourceuser

ENTRYPOINT [ "resource-server" ]
