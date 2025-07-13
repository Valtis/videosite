FROM rust:1.88-alpine3.22 AS builder

RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    openssl-libs-static \
    pkgconf \
    libpq-dev
    
COPY ./Cargo.toml /app/Cargo.toml
COPY ./Cargo.lock /app/Cargo.lock

# -- caching dependencies
# Create a dummy main.rs for cargo to build --something--, 
# build the application, then remove the dummy main.rs
# and the final cached binary (so cargo build will not reuse the cached binary) 
RUN mkdir -p /app/src && \
    echo 'fn main() { println!("Hello, world!"); }' > /app/src/main.rs && \
    cd /app && \
    cargo build --release --target x86_64-unknown-linux-musl && \
    cargo clean --target x86_64-unknown-linux-musl -p resource-server && \
    rm -rf /app/src && \
    rm -rf /app/target/x86_64-unknown-linux-musl/release/deps/resource-server*

# -- end caching dependencies


COPY ./src /app/src

WORKDIR /app

RUN ls /app/target/x86_64-unknown-linux-musl/release -la
RUN touch /app/src/main.rs && cargo build --release --target x86_64-unknown-linux-musl


FROM alpine:3.22

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/resource-server /usr/local/bin/resource-server

# Create a non-root user without login priviledges to reduce the potential attack surface somewhat
RUN addgroup -g 1000 resourceuser && \
    adduser -D -u 1000 -G resourceuser resourceuser && \
    chown resourceuser:resourceuser /usr/local/bin/resource-server

USER resourceuser

ENTRYPOINT [ "resource-server" ]
