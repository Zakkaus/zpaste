#!/bin/bash
# Self-test of scripts/check-version-tag.sh: plant a violation and expect the
# check to turn red, then give it the right tag and expect green. A check that
# cannot fail would pass anything.
#
# Exit codes: 0 every case behaved, 1 a case did not, 2 the test could not run.
# Usage: test_check_version_tag.sh

set -o nounset
set -o pipefail

here="$(cd "$(dirname "$0")" && pwd)" || exit 2
check="$here/check-version-tag.sh"
manifest="$here/../Cargo.toml"

if ! version="$(cargo metadata --no-deps --format-version=1 --manifest-path "$manifest" \
    | jq -er '.packages[] | select(.name == "wastebin") | .version')"; then
    echo "cannot read the version from $manifest" >&2
    exit 2
fi

failed=0

# expect NAME WANT_EXIT COMMAND...: run COMMAND, compare its exit code.
expect() {
    local name="$1" want="$2" got
    shift 2
    "$@" >/dev/null 2>&1
    got=$?
    if [ "$got" -ne "$want" ]; then
        echo "FAIL $name: exit $got, want $want" >&2
        failed=1
    else
        echo "ok   $name: exit $got"
    fi
}

expect "tag v$version" 0 "$check" "v$version"
expect "tag without v" 1 "$check" "$version"
expect "tag v3.9" 1 "$check" "v3.9"
expect "tag of another patch version" 1 "$check" "v$version.1"
expect "tag of the version plus a suffix" 1 "$check" "v$version-rc1"
expect "empty tag" 2 "$check" ""
expect "no argument" 2 "$check"

# A copy of the script in a scratch repository: with no Cargo.toml one level up
# it cannot read a version; with a planted version it checks that version.
lonely="$(mktemp -d)" || exit 2
mkdir "$lonely/scripts" "$lonely/src"
cp "$check" "$lonely/scripts/check-version-tag.sh"
touch "$lonely/src/lib.rs"
copy="$lonely/scripts/check-version-tag.sh"
expect "missing Cargo.toml" 2 "$copy" "v$version"

# plant VERSION: write a Cargo.toml whose package wastebin has that version.
plant() {
    printf '[package]\nname = "wastebin"\nversion = "%s"\nedition = "2021"\n' "$1" \
        > "$lonely/Cargo.toml"
}
plant "0.1.0-beta.1"
expect "prerelease tag v0.1.0-beta.1" 0 "$copy" "v0.1.0-beta.1"
expect "prerelease version, tag without suffix" 1 "$copy" "v0.1.0"
expect "prerelease version, tag of another prerelease" 1 "$copy" "v0.1.0-beta.2"
expect "prerelease version, tag without v" 1 "$copy" "0.1.0-beta.1"
expect "prerelease version, short tag" 1 "$copy" "v0.1"
plant "0.1.0"
expect "release version, prerelease tag" 1 "$copy" "v0.1.0-beta.1"
plant "0.1.0-beta.1+build5"
expect "build metadata in the version" 1 "$copy" "v0.1.0-beta.1+build5"
expect "build metadata, tag without it" 1 "$copy" "v0.1.0-beta.1"
rm -r "${lonely:?}"

exit "$failed"
