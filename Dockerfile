# adapted from
# <https://dev.to/deciduously/use-multi-stage-docker-builds-for-statically-linked-rust-binaries-3jgd>
FROM rust:1.45.2-alpine AS builder
ENV USER=backend
WORKDIR /usr/src
RUN apk add openssl-dev musl-dev
RUN cargo new backend
WORKDIR /usr/src/backend
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release

COPY migrations src tests ./
RUN cargo install --path .

FROM scratch
COPY --from=builder /usr/local/cargo/bin/backend .
USER 1000
CMD ["./backend"]
