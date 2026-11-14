#!/bin/bash
# Copy the static x86_64 musl executables (zpaste, wastebin-ctl) of the
# Dockerfile's amd64 target into DIR. release.yml runs this on the image it
# just pushed, so a release ships the very executables that image runs.
# Without --image the script builds the amd64 target first; it works with or
# without BuildKit, and extra arguments go to docker build, for example
# --network host.
#
# Exit codes: 0 exported, 1 the build or the copy failed, 2 could not start.
# Usage: export-executables.sh DIR --image IMAGE
#        export-executables.sh DIR [docker build argument...]

set -o nounset
set -o pipefail

if [ "$#" -lt 1 ] || [ -z "$1" ]; then
    echo "usage: $0 DIR --image IMAGE | $0 DIR [docker build argument...]" >&2
    exit 2
fi
dest="$1"
shift
image=""
built=false
if [ "${1:-}" = "--image" ]; then
    if [ "$#" -ne 2 ] || [ -z "$2" ]; then
        echo "usage: $0 DIR --image IMAGE" >&2
        exit 2
    fi
    image="$2"
fi
if ! command -v docker >/dev/null || ! docker info >/dev/null 2>&1; then
    echo "cannot reach a docker daemon" >&2
    exit 2
fi
root="$(cd "$(dirname "$0")/.." && pwd)" || exit 2
mkdir -p "$dest" || exit 2

if [ -z "$image" ]; then
    image="zpaste-export:$$"
    built=true
    docker build --target amd64 --tag "$image" "$@" "$root" || exit 1
fi
container="$(docker create "$image")" || exit 1
status=0
for exe in zpaste wastebin-ctl; do
    docker cp "$container:/app/$exe" "$dest/$exe" || status=1
done
docker rm "$container" >/dev/null
if [ "$built" = true ]; then
    docker rmi "$image" >/dev/null
fi
[ "$status" -eq 0 ] && echo "exported zpaste and wastebin-ctl from $image to $dest"
exit "$status"
