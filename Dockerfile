# syntax=docker/dockerfile:1
FROM debian:bookworm-slim AS installer

# Wings replaces Cmd, but preserves the image user and entrypoint. Installation
# needs root to initialize volume ownership and must start its shell directly.
USER 0:0
WORKDIR /mnt/server
COPY server/app/config.template.toml /usr/local/share/ferrumc/config.template.toml

FROM rust:1.99.0-bookworm AS builder

# The repository selects stable; pin the build to this image's numeric toolchain.
ENV RUSTUP_TOOLCHAIN=1.99.0
WORKDIR /build
COPY rust-toolchain.toml ./
COPY fixtures ./fixtures
COPY server ./server
WORKDIR /build/server
RUN cargo build --release --locked -p ferrumc-app

FROM debian:bookworm-slim AS runtime

# glibc matches the build stage and supports trusted Linux native plugin bundles.
RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates libgcc-s1 \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 1000 container \
    && useradd --uid 1000 --gid 1000 --home-dir /home/container --create-home --shell /bin/sh container \
    && mkdir -p /usr/local/share/ferrumc /home/container/world /home/container/plugins \
    && chown 1000:1000 /home/container /home/container/world /home/container/plugins

COPY --from=builder /build/server/target/release/ferrumc /usr/local/bin/ferrumc
COPY server/app/config.template.toml /usr/local/share/ferrumc/config.template.toml
COPY --chmod=755 deploy/docker-entrypoint.sh /usr/local/bin/ferrumc-entrypoint

ENV SERVER_PORT=25565 \
    RUST_LOG=info
WORKDIR /home/container
USER 1000:1000
EXPOSE 25565/tcp
STOPSIGNAL SIGTERM

# The binary and template remain image-owned when /home/container is mounted.
ENTRYPOINT ["/usr/local/bin/ferrumc-entrypoint"]
CMD ["--config", "/home/container/config.toml"]
