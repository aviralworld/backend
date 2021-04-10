ARG BASE_IMAGE
ARG FFMPEG_VERSION

FROM $BASE_IMAGE AS builder

ARG TIMESTAMP
ENV BUILD_TIMESTAMP=$TIMESTAMP
ARG REVISION
ENV BACKEND_REVISION=$REVISION

WORKDIR /home/rust/src/backend

# build project
COPY info ./info
COPY log ./log
COPY server ./server

# without the `touch`, the compiler doesn't appear to realize the code has changed
RUN touch info/src/lib.rs && touch log/src/lib.rs && cargo build --target x86_64-unknown-linux-musl --bin backend --release --frozen --offline

FROM mwader/static-ffmpeg:$FFMPEG_VERSION AS ffmpeg

FROM scratch

ARG TIMESTAMP
ARG REVISION
LABEL timestamp=$TIMESTAMP revision=$REVISION

COPY --from=ffmpeg /ffprobe /bin/ffprobe
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
ENV BACKEND_FFPROBE_PATH=/bin/ffprobe
ENV SSL_CERT_DIR=/etc/ssl/certs/
COPY --from=builder /home/rust/src/backend/target/x86_64-unknown-linux-musl/release/backend /usr/app/backend
ENV TMPDIR /usr/app/tmp
CMD ["/usr/app/backend"]
