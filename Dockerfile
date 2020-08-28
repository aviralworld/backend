# TODO adopt multi-stage Dockerfile for statically linked build

# this was initially based on
# <https://dev.to/deciduously/use-multi-stage-docker-builds-for-statically-linked-rust-binaries-3jgd>,
# but somewhere along the way some library caused problems with static
# linking and Alpine Linux

FROM rust:1.45.2 AS builder
ARG BACKEND_REVISION
ENV USER=backend
ENV BACKEND_REVISION=$BACKEND_REVISION
ENV CARGO_INCREMENTAL=0
WORKDIR /usr/src

# TODO remove ffmpeg once `ffmpeg-next` is integrated
RUN apt-get update -qqy && apt-get -qqy install libssl-dev ffmpeg

RUN cargo new backend
WORKDIR /usr/src/backend
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release --locked

COPY src ./src
RUN cargo install --path . --frozen --offline

CMD ["/usr/local/cargo/bin/backend"]
