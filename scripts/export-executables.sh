#!/bin/bash
# Build the Dockerfile's amd64 target and copy its static x86_64 musl
# executables (zpaste, wastebin-ctl) into DIR, so a release ships the same
# executables the image runs. release.yml runs this; it works with or without
# BuildKit. Extra arguments go to docker build, for example --network host.
#
# Exit codes: 0 exported, 1 the build or the copy failed, 2 could not start.
# Usage: export-executables.sh DIR [docker build argument...]

set -o nounset
set -o pipefail

if [ "$#" -lt 1 ] || [ -z "$1" ]; then
    echo "usage: $0 DIR [docker build argument...]" >&2
    exit 2
fi
dest="$1"
shift
if ! command -v docker >/dev/null || ! docker info >/dev/null 2>&1; then
    echo "cannot reach a docker daemon" >&2
    exit 2
fi
root="$(cd "$(dirname "$0")/.." && pwd)" || exit 2
mkdir -p "$dest" || exit 2

image="zpaste-export:$$"
docker build --target amd64 --tag "$image" "$@" "$root" || exit 1
container="$(docker create "$image")" || exit 1
status=0
for exe in zpaste wastebin-ctl; do
    docker cp "$container:/app/$exe" "$dest/$exe" || status=1
done
docker rm "$container" >/dev/null
docker rmi "$image" >/dev/null
[ "$status" -eq 0 ] && echo "exported zpaste and wastebin-ctl to $dest"
exit "$status"
