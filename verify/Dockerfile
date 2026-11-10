# arks M1 clean-room toolchain image.
# Base: official Rust on Debian bookworm + stock apt git (no host git, no gix).
# Pin must match verify/images.lock.json (toolchain-base digest).
FROM rust:1.99.1-bookworm@sha256:e6f2cff35d0f309acf8433b4f05878ed77092602c5065cc81801177da0bb3a9c

# openssh-client/server: M16 Sy-4 client SSH loopback fixture (G21).
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        git \
        ca-certificates \
        openssh-client \
        openssh-server \
    && rm -rf /var/lib/apt/lists/* \
    && git --version | tee /etc/arks-stock-git-version.txt \
    && rustc --version | tee /etc/arks-rustc-version.txt \
    && ssh -V 2>&1 | tee /etc/arks-openssh-version.txt \
    && mkdir -p /var/run/sshd

ENV CARGO_BUILD_JOBS=2
WORKDIR /workspace
