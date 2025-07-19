FROM rust:1.88-alpine3.22 AS builder

RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    openssl-libs-static \
    pkgconf \
    libpq-dev

COPY libs/audit /app/libs/audit
    
COPY auth/Cargo.toml /app/auth/Cargo.toml
COPY auth/Cargo.lock /app/auth/Cargo.lock

# -- caching dependencies
# Create a dummy main.rs for cargo to build --something--, 
# build the application, then remove the dummy main.rs
# and the final cached binary (so cargo build will not reuse the cached binary) 
RUN mkdir -p /app/auth/src && \
    echo 'fn main() { println!("Hello, world!"); }' > /app/auth/src/main.rs && \
    cd /app/auth && \
    cargo build --release --target x86_64-unknown-linux-musl && \
    cargo clean --target x86_64-unknown-linux-musl -p auth && \
    rm -rf /app/auth/src && \
    rm -rf /app/auth/target/x86_64-unknown-linux-musl/release/deps/auth*

# -- end caching dependencies


COPY auth/src /app/auth/src

WORKDIR /app/auth

RUN touch /app/auth/src/main.rs && cargo build --release --target x86_64-unknown-linux-musl


FROM alpine:3.22

COPY --from=builder /app//auth/target/x86_64-unknown-linux-musl/release/auth /usr/local/bin/auth

# Create a non-root user without login priviledges to reduce the potential attack surface somewhat
RUN addgroup -g 1000 authuser && \
    adduser -D -u 1000 -G authuser authuser && \
    chown authuser:authuser /usr/local/bin/auth

USER authuser

ENTRYPOINT [ "auth" ]
