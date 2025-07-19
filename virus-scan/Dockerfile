FROM rust:1.88-alpine3.22 AS builder

RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    openssl-libs-static \
    pkgconf \
    libpq-dev
    
COPY ./libs/audit /app/libs/audit

COPY ./virus-scan/Cargo.toml /app/virus-scan/Cargo.toml
COPY ./virus-scan/Cargo.lock /app/virus-scan/Cargo.lock

# -- caching dependencies
# Create a dummy main.rs for cargo to build --something--, 
# build the application, then remove the dummy main.rs
# and the final cached binary (so cargo build will not reuse the cached binary) 
RUN mkdir -p /app/virus-scan/src && \
    echo 'fn main() { println!("Hello, world!"); }' > /app/virus-scan/src/main.rs && \
    cd /app/virus-scan && \
    cargo build --release --target x86_64-unknown-linux-musl && \
    cargo clean --target x86_64-unknown-linux-musl -p virus-scan && \
    rm -rf /app/virus-scan/src && \
    rm -rf /app/virus-scan/target/x86_64-unknown-linux-musl/release/deps/virus-scan*

# -- end caching dependencies


COPY ./virus-scan/src /app/virus-scan/src

WORKDIR /app/virus-scan

RUN touch /app/virus-scan/src/main.rs && cargo build --release --target x86_64-unknown-linux-musl

FROM clamav/clamav:1.4.3
#FROM alpine:3.22

COPY --from=builder /app/virus-scan/target/x86_64-unknown-linux-musl/release/virus-scan /usr/local/bin/virus-scan

# Create a non-root user without login priviledges to reduce the potential attack surface somewhat
RUN addgroup -g 1000 virusscanuser && \
    adduser -D -u 1000 -G virusscanuser virusscanuser && \
    chown virusscanuser:virusscanuser /usr/local/bin/virus-scan

# Run as root for now, as clamav needs root to run. The script will switch to the non-root user later.

COPY ./virus-scan/entrypoint.sh /usr/local/bin/entrypoint.sh
ENTRYPOINT [ "/usr/local/bin/entrypoint.sh" ]
