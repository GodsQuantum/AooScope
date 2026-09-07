FROM debian:trixie-slim

# Installer les dépendances système
RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    pkg-config \
    libudev-dev \
    git \
    python3 \
    python3-pip \
    python3-pil \
    python3-flask \
    python3-flask-cors \
    iproute2 \
    procps \
    && rm -rf /var/lib/apt/lists/*

# Installer Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Compiler asterctl depuis les sources
RUN git clone https://github.com/zehnm/aoostar-rs.git /build/aoostar-rs && \
    cd /build/aoostar-rs && \
    cargo build --release && \
    cp target/release/asterctl /usr/local/bin/ && \
    cp target/release/aster-sysinfo /usr/local/bin/ && \
    rm -rf /build

# Créer les dossiers
RUN mkdir -p /app/cfg/sensors /app/fonts

# Copier les fichiers de config
COPY cfg/ /app/cfg/
COPY webui.py /app/
COPY proxmox-sensors.sh /app/
COPY start.sh /app/

RUN chmod +x /app/proxmox-sensors.sh /app/start.sh

WORKDIR /app

EXPOSE 8765

CMD ["/app/start.sh"]
