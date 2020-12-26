ARG RUST_VERSION

FROM ekidd/rust-musl-builder:$RUST_VERSION AS builder

ARG TIMESTAMP
ENV BUILD_TIMESTAMP=$TIMESTAMP
ARG REVISION
ENV BACKEND_REVISION=$REVISION

ENV CARGO_INCREMENTAL=0

ENV USER=rust

WORKDIR /home/rust/src
RUN cargo new backend
WORKDIR /home/rust/src/backend

# build dependencies
COPY Cargo.toml Cargo.lock ./
RUN OPENSSL_STATIC=1 cargo build --target x86_64-unknown-linux-musl --release --locked

# build project
COPY src ./src
RUN OPENSSL_STATIC=1 cargo build --target x86_64-unknown-linux-musl --release --frozen --offline

FROM mwader/static-ffmpeg:4.3.1 AS ffmpeg

FROM scratch

ARG TIMESTAMP
ARG REVISION
LABEL timestamp=$TIMESTAMP revision=$REVISION
COPY --from=ffmpeg /ffprobe /bin/ffprobe
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
ENV BACKEND_FFPROBE_PATH=/bin/ffprobe
ENV SSL_CERT_DIR=/etc/ssl/certs/
COPY --from=builder /home/rust/src/backend/target/x86_64-unknown-linux-musl/release/backend /usr/app/backend
USER 1000
CMD ["/usr/app/backend"]
