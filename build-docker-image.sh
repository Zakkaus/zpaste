#!/bin/bash
# Build the x86_64 image locally, the same Dockerfile target that image.yml
# builds in CI. CI alone pushes to ghcr.io/zakkaus/zpaste; this script never
# pushes. Extra arguments go to docker build, for example
# `./build-docker-image.sh --network host` on rootless Docker.
#
# Exit codes: 0 built, 1 the build failed, 2 the build could not start.
# Usage: build-docker-image.sh [docker build argument...]

set -o nounset
set -o pipefail

if ! command -v docker >/dev/null; then
    echo "docker not found" >&2
    exit 2
fi
if ! docker info >/dev/null 2>&1; then
    echo "cannot reach the docker daemon of context $(docker context show 2>/dev/null)" >&2
    exit 2
fi
root="$(cd "$(dirname "$0")" && pwd)" || exit 2
tag="zpaste:$(git -C "$root" describe --tags --always --dirty)" || exit 2

docker build --target amd64 --tag "$tag" "$@" "$root" || exit 1
echo "built $tag"
