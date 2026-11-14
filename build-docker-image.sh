#!/bin/bash

set -o errexit
set -o nounset
set -o pipefail

VERSION=$(git describe --tags)

docker build --target amd64 -t ghcr.io/zakkaus/zpaste:${VERSION}-amd64 .
docker build --target arm64 -t ghcr.io/zakkaus/zpaste:${VERSION}-arm64 .

docker push ghcr.io/zakkaus/zpaste:${VERSION}-amd64
docker push ghcr.io/zakkaus/zpaste:${VERSION}-arm64

docker manifest create ghcr.io/zakkaus/zpaste:${VERSION} \
    ghcr.io/zakkaus/zpaste:${VERSION}-amd64 \
    ghcr.io/zakkaus/zpaste:${VERSION}-arm64

docker manifest annotate ghcr.io/zakkaus/zpaste:${VERSION} \
    ghcr.io/zakkaus/zpaste:${VERSION}-arm64 \
    --os linux \
    --arch arm64

docker manifest rm ghcr.io/zakkaus/zpaste:latest 2>/dev/null || true

docker manifest create ghcr.io/zakkaus/zpaste:latest \
    ghcr.io/zakkaus/zpaste:${VERSION}-amd64 \
    ghcr.io/zakkaus/zpaste:${VERSION}-arm64

docker manifest annotate ghcr.io/zakkaus/zpaste:latest \
    ghcr.io/zakkaus/zpaste:${VERSION}-arm64 \
    --os linux \
    --arch arm64

docker manifest inspect ghcr.io/zakkaus/zpaste:${VERSION}

docker manifest push ghcr.io/zakkaus/zpaste:${VERSION}
docker manifest push ghcr.io/zakkaus/zpaste:latest
