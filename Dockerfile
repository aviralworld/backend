# adapted from
# <https://dev.to/deciduously/use-multi-stage-docker-builds-for-statically-linked-rust-binaries-3jgd>
FROM rust:1.45.2-alpine AS builder
ENV USER=backend
ENV CARGO_INCREMENTAL=0
WORKDIR /usr/src

# TODO remove ffmpeg once `ffmpeg-next` is integrated
RUN apk add --quiet openssl-dev musl-dev ffmpeg

RUN cargo new backend
WORKDIR /usr/src/backend
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release

COPY src ./
RUN cargo install --path . --frozen --offline

FROM scratch
COPY --from=builder /usr/local/cargo/bin/backend .
USER 1000
CMD ["./backend"]
