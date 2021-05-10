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

# We can’t switch to an unprivileged user (e.g. UID 1000) because the
# temporary directory cannot be created when this image is run as a
# service to test the frontend. GitLab CI would have to support
# setting up services that use volumes with specific permissions.

ARG TIMESTAMP
ARG REVISION
LABEL timestamp=$TIMESTAMP revision=$REVISION

COPY --from=ffmpeg /ffprobe /bin/ffprobe
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder /home/rust/src/backend/target/x86_64-unknown-linux-musl/release/backend /usr/app/backend
ENV BACKEND_FFPROBE_PATH=/bin/ffprobe
ENV SSL_CERT_DIR=/etc/ssl/certs/
ENV TMPDIR /usr/app/tmp
CMD ["/usr/app/backend"]
