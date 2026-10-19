FROM debian@sha256:a29215f6a35e51e22adffa17f89e9d2ef06214e64a2bad10d765c46aea49f11f AS runtime-base
RUN export http_proxy="$HTTP_PROXY" https_proxy="$HTTPS_PROXY"; \
    apt-get update && apt-get install -y --no-install-recommends ca-certificates libssl3t64 ffmpeg \
    && rm -rf /var/lib/apt/lists/*
FROM runtime-base
ARG SOURCE_REVISION
ARG BINARY_SHA256
LABEL org.opencontainers.image.title="biliup custom danmaku runtime" \
      org.opencontainers.image.revision=$SOURCE_REVISION \
      biliup.upstream="3d884a570be7761e65413785cfc63d88a97ef0e5" \
      biliup.binary.sha256=$BINARY_SHA256 \
      biliup_local_test="opt-in"
COPY biliup /usr/local/bin/biliup
ENV TZ=Asia/Shanghai LANG=C.UTF-8 BILIUP_LOCAL_DANMAKU_TEST=0 RUST_LOG=info,danmaku=debug
WORKDIR /opt
EXPOSE 19159
ENTRYPOINT ["biliup"]
CMD ["server", "--bind", "0.0.0.0", "--auth"]
