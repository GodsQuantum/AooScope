FROM rust:trixie AS builder
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates git pkg-config libudev-dev \
    && rm -rf /var/lib/apt/lists/*
ARG AOOSTAR_RS_REF=2f4d95957d2d61f9fe5cd27e4cf14bd2ae566f63
RUN git clone https://github.com/zehnm/aoostar-rs.git /src/aoostar-rs \
    && cd /src/aoostar-rs \
    && git checkout "$AOOSTAR_RS_REF" \
    && cargo build --release --locked \
    && install -Dm755 target/release/asterctl /out/asterctl \
    && install -Dm755 target/release/aster-sysinfo /out/aster-sysinfo

FROM debian:trixie-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libudev1 python3 python3-pil python3-flask python3-flask-cors \
    iproute2 procps tini \
    && rm -rf /var/lib/apt/lists/*
RUN mkdir -p /app/cfg/sensors /run/aooscope-secrets
COPY --from=builder /out/asterctl /usr/local/bin/asterctl
COPY --from=builder /out/aster-sysinfo /usr/local/bin/aster-sysinfo
COPY --from=builder /src/aoostar-rs/fonts/ /app/fonts/
COPY cfg/ /app/cfg/
COPY aooscope/ /app/aooscope/
COPY webui.py start.sh /app/
RUN chmod 0755 /app/start.sh
WORKDIR /app
ENV PYTHONPATH=/app PYTHONUNBUFFERED=1
EXPOSE 8765
ENTRYPOINT ["/usr/bin/tini", "-g", "--"]
CMD ["/app/start.sh"]
