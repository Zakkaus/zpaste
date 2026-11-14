# Changelog

Notable changes to Zpaste. A release tag `vX.Y.Z` publishes the section headed
`[X.Y.Z]` as the release notes and fails when that section is missing, so add it
before tagging. wastebin's own changes up to commit `c0ec968` are in
[docs/upstream/CHANGELOG.md](docs/upstream/CHANGELOG.md).

## [Unreleased]

### Changed

- The server executable is named `zpaste`, and log lines name `zpaste` as their
  target. Configuration variables keep the `WASTEBIN_` prefix.
- The container image runs as uid and gid 10001.
- CI builds the image on every pull request and pushes it to
  `ghcr.io/zakkaus/zpaste` from main and from version tags. A version tag also
  attaches the static x86_64 musl executables to its GitHub release.
