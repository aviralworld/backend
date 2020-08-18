# adapted from
# <https://dev.to/deciduously/use-multi-stage-docker-builds-for-statically-linked-rust-binaries-3jgd>
FROM rust:1.45.2-alpine AS builder
ARG BACKEND_REVISION
ENV USER=backend
ENV BACKEND_REVISION=$BACKEND_REVISION
ENV CARGO_INCREMENTAL=0
WORKDIR /usr/src

# TODO remove ffmpeg once `ffmpeg-next` is integrated
RUN apk add --quiet openssl-dev musl-dev ffmpeg
RUN rustup target add x86_64-unknown-linux-musl

RUN cargo new backend
WORKDIR /usr/src/backend
COPY Cargo.toml Cargo.lock ./
RUN cargo build --target x86_64-unknown-linux-musl --release --locked

COPY src ./src
RUN cargo install --target x86_64-unknown-linux-musl --path . --frozen --offline

FROM scratch
LABEL BACKEND_REVISION=$BACKEND_REVISION
COPY --from=builder /usr/local/cargo/bin/backend .
USER 1000
CMD ["./backend"]
