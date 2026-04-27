# syntax=docker/dockerfile:1.7

FROM rustlang/rust:nightly-bookworm AS builder

WORKDIR /build/mira-oxide

COPY mira-oxide/Cargo.toml mira-oxide/Cargo.lock ./
COPY mira-oxide/src ./src

RUN cargo build --release

FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get -o Acquire::Check-Valid-Until=false -o Acquire::Check-Date=false update \
    && apt-get install -y --no-install-recommends \
        bash \
        ca-certificates \
        curl \
        fuse-overlayfs \
        openjdk-17-jre-headless \
        podman \
        procps \
        slirp4netns \
        uidmap \
    && rm -rf /var/lib/apt/lists/*

RUN curl -fsSL https://get.nextflow.io | bash \
    && mv nextflow /usr/local/bin/nextflow \
    && chmod +x /usr/local/bin/nextflow

RUN mkdir -p /etc/containers \
    && printf '[storage]\ndriver = "vfs"\nrunroot = "/var/run/containers/storage"\ngraphroot = "/var/lib/containers/storage"\n' > /etc/containers/storage.conf

COPY --from=builder /build/mira-oxide/target/release/mira-oxide /usr/local/bin/mira-oxide
COPY MIRA-NF /opt/MIRA-NF

ENV MIRA_NF_DIR=/opt/MIRA-NF \
    MIRA_UI_DATA_ROOT=/workspace \
    MIRA_UI_STATE_DIR=/var/lib/mira-oxide-ui

WORKDIR /app

RUN mkdir -p /workspace /var/lib/mira-oxide-ui

VOLUME ["/workspace", "/var/lib/mira-oxide-ui"]

EXPOSE 3000

CMD ["mira-oxide", "serve", "--listen", "0.0.0.0:3000", "--data-root", "/workspace", "--pipeline-dir", "/opt/MIRA-NF", "--state-dir", "/var/lib/mira-oxide-ui"]
