FROM rust:1.88-alpine3.22 AS builder

RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    openssl-libs-static \
    pkgconf \
    libpq-dev

COPY embed-tag-service/Cargo.toml /app/embed-tag-service/Cargo.toml
COPY embed-tag-service/Cargo.lock /app/embed-tag-service/Cargo.lock

# -- caching dependencies
# Create a dummy main.rs for cargo to build --something--, 
# build the application, then remove the dummy main.rs
# and the final cached binary (so cargo build will not reuse the cached binary) 
RUN mkdir -p /app/embed-tag-service/src && \
    echo 'fn main() { println!("Hello, world!"); }' > /app/embed-tag-service/src/main.rs && \
    cd /app/embed-tag-service && \
    cargo build --release --target x86_64-unknown-linux-musl && \
    cargo clean --target x86_64-unknown-linux-musl -p embed-tag-service && \
    rm -rf /app/embed-tag-service/src && \
    rm -rf /app/embed-tag-service/target/x86_64-unknown-linux-musl/release/deps/embed-tag-service*

# -- end caching dependencies


COPY embed-tag-service/src /app/embed-tag-service/src

WORKDIR /app/embed-tag-service

RUN touch /app/embed-tag-service/src/main.rs && cargo build --release --target x86_64-unknown-linux-musl


FROM alpine:3.22

COPY --from=builder /app/embed-tag-service/target/x86_64-unknown-linux-musl/release/embed-tag-service /usr/local/bin/embed-tag-service

# Create a non-root user without login priviledges to reduce the potential attack surface somewhat
RUN addgroup -g 1000 embed-tag-serviceuser && \
    adduser -D -u 1000 -G embed-tag-serviceuser embed-tag-serviceuser && \
    chown embed-tag-serviceuser:embed-tag-serviceuser /usr/local/bin/embed-tag-service

USER embed-tag-serviceuser

ENTRYPOINT [ "embed-tag-service" ]
