#!/bin/bash
# Fail when a release tag does not name the version the server reports.
#
# Zpaste's version is [workspace.package] version in the root Cargo.toml; the
# server package (still named wastebin) inherits it. A release tag is "v"
# followed by that version, for example v3.9.0.
#
# Exit codes: 0 the tag matches, 1 it does not, 2 the check could not run.
# Usage: check-version-tag.sh TAG

set -o nounset
set -o pipefail

if [ "$#" -ne 1 ] || [ -z "$1" ]; then
    echo "usage: $0 TAG" >&2
    exit 2
fi
tag="$1"
manifest="$(dirname "$0")/../Cargo.toml"

if ! version="$(cargo metadata --no-deps --format-version=1 --manifest-path "$manifest" \
    | jq -er '.packages[] | select(.name == "wastebin") | .version')"; then
    echo "cannot read the version of package wastebin from $manifest" >&2
    exit 2
fi

if [ "$tag" != "v$version" ]; then
    echo "tag $tag does not match version $version in Cargo.toml; expected v$version" >&2
    exit 1
fi
echo "tag $tag matches version $version"
