FROM rust:1.88-alpine3.22 AS builder

RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    openssl-libs-static \
    pkgconf \
    libpq-dev \
    clamav
    
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
    cargo clean --target x86_64-unknown-linux-musl -p virus-scan && \
    rm -rf /app/src && \
    rm -rf /app/target/x86_64-unknown-linux-musl/release/deps/virus-scan*

# -- end caching dependencies


COPY ./src /app/src

WORKDIR /app

RUN ls /app/target/x86_64-unknown-linux-musl/release -la
RUN touch /app/src/main.rs && cargo build --release --target x86_64-unknown-linux-musl

FROM clamav/clamav:1.4.3
#FROM alpine:3.22

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/virus-scan /usr/local/bin/virus-scan

COPY entrypoint.sh /usr/local/bin/entrypoint.sh
ENTRYPOINT [ "/usr/local/bin/entrypoint.sh" ]
