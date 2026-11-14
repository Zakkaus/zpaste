# Cross-compile Dockerfile supporting both x86_64-unknown-linux-musl and
# aarch64-unknown-linux-musl targets using zig to link against musl libc. Note
# that this is to be used from an x86_64 host.
#
# The web stage builds web/ (the reading page); the builder embeds its
# web/dist in the executable and refuses to build without it
# (ZPASTE_REQUIRE_WEB, crates/wastebin_server/build.rs).
#
# Every input of both build stages is pinned, because the release ships the
# executables built here: the base images by digest, pnpm by version and the
# web dependencies by web/pnpm-lock.yaml, zig by the sha256 that ziglang.org
# publishes, cargo-zigbuild by version. Move them together, and record the new
# tag, checksums and version in the comments below.
#
# CARGO_TARGETS lists the targets to build; the default builds only the one
# the amd64 stage needs. The arm64 stage needs
# --build-arg CARGO_TARGETS=aarch64-unknown-linux-musl.

# --- web app

# node:24.22.3-bookworm-slim (index digest of the tag when it was pinned)
FROM node:24.22.3-bookworm-slim@sha256:dc26ae92c9fb15cf6b1d63e20f0944b2efe0f115b20e00be48fb23fceb061c54 AS web

# The version web/package.json names in packageManager.
RUN npm install --global pnpm@11.15.1

WORKDIR /web
COPY web/package.json web/pnpm-lock.yaml web/pnpm-workspace.yaml ./
RUN pnpm install --frozen-lockfile
COPY web/ ./
RUN pnpm build

# --- build image

# rust:1.95 (index digest of the tag when it was pinned)
FROM rust:1.95@sha256:f49565f188ee00bc2a18dd418183f2c5f23ef7d6e691890517ed341a598f67c3 AS builder

ARG CARGO_TARGETS="x86_64-unknown-linux-musl"

RUN set -eu; for t in $CARGO_TARGETS; do rustup target add "$t"; done

RUN update-ca-certificates

# zig-<arch>-linux-0.15.2.tar.xz, sha256 from https://ziglang.org/download/index.json
ENV ZIGVERSION=0.15.2
ENV ZIG_SHA256_X86_64=02aa270f183da276e5b5920b1dac44a63f1a49e55050ebde3aecc9eb82f93239
ENV ZIG_SHA256_AARCH64=958ed7d1e00d0ea76590d27666efbf7a932281b3d7ba0c6b01b0ff26498f667f

RUN set -eu; \
    ARCH=$(uname -m); \
    if [ "$ARCH" = "aarch64" ]; then ZIGARCH="aarch64"; ZIGSUM="$ZIG_SHA256_AARCH64"; \
    else ZIGARCH="x86_64"; ZIGSUM="$ZIG_SHA256_X86_64"; fi; \
    TARBALL="zig-$ZIGARCH-linux-$ZIGVERSION.tar.xz"; \
    wget "https://ziglang.org/download/$ZIGVERSION/$TARBALL"; \
    echo "$ZIGSUM  $TARBALL" | sha256sum -c -; \
    tar -C /usr/local --strip-components=1 -xf "$TARBALL"; \
    mv /usr/local/zig /usr/local/bin; \
    rm "$TARBALL"

RUN cargo install --locked --version 0.24.1 cargo-zigbuild

WORKDIR /app

COPY . .
COPY --from=web /web/dist web/dist
ENV ZPASTE_REQUIRE_WEB=1

RUN set -eu; \
    args=""; \
    for t in $CARGO_TARGETS; do args="$args --target $t"; done; \
    cargo zigbuild --locked --release $args --bin zpaste --bin wastebin-ctl

# uid and gid 10001 match the live compose file (user: "10001:10001"), which
# owns the data volume.
RUN addgroup --gid "10001" "app" && \
    adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "10001" \
    --gid "10001" \
    "app"


# --- x86_64-unknown-linux-musl final image

FROM scratch AS amd64

COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

WORKDIR /app
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/zpaste ./
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/wastebin-ctl ./
USER 10001:10001
CMD ["/app/zpaste"]

# --- aarch64-unknown-linux-musl final image

FROM scratch AS arm64

COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

WORKDIR /app
COPY --from=builder /app/target/aarch64-unknown-linux-musl/release/zpaste ./
COPY --from=builder /app/target/aarch64-unknown-linux-musl/release/wastebin-ctl ./
USER 10001:10001
CMD ["/app/zpaste"]
