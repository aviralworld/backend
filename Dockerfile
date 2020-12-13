ARG RUST_VERSION

FROM ekidd/rust-musl-builder:$RUST_VERSION AS builder

ARG BUILD_TIMESTAMP
ENV BUILD_TIMESTAMP=$BUILD_TIMESTAMP
ARG BACKEND_REVISION
ENV BACKEND_REVISION=$BACKEND_REVISION
ENV CARGO_INCREMENTAL=0
ENV USER=rust

WORKDIR /home/rust/src
RUN cargo new backend
WORKDIR /home/rust/src/backend
COPY Cargo.toml Cargo.lock ./
RUN OPENSSL_LIB_DIR=/usr/local/musl/lib/ OPENSSL_INCLUDE_DIR=/usr/local/musl/include OPENSSL_STATIC=1 cargo build --target x86_64-unknown-linux-musl --release --locked

COPY src ./src
RUN OPENSSL_LIB_DIR=/usr/local/musl/lib/ OPENSSL_INCLUDE_DIR=/usr/local/musl/include OPENSSL_STATIC=1 cargo build --target x86_64-unknown-linux-musl --release --frozen --offline

FROM scratch
# TODO add ffmpeg
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
ENV SSL_CERT_DIR=/etc/ssl/certs/
COPY --from=builder /home/rust/src/backend/target/x86_64-unknown-linux-musl/release/backend /usr/app/backend
USER 1000
CMD ["/usr/app/backend"]
