# Changelog

Notable changes to Zpaste. A release tag `vX.Y.Z` (or `vX.Y.Z-beta.N`)
publishes the section headed `[X.Y.Z]` as the release notes and fails when that
section is missing, so add it before tagging. wastebin's own changes up to commit `c0ec968` are in
[docs/upstream/CHANGELOG.md](docs/upstream/CHANGELOG.md).

## [0.1.0-beta.1]

The first Zpaste version is a beta. Its tag `v0.1.0-beta.1` publishes a GitHub
prerelease and the image tag `0.1.0-beta.1`.

### Added

- Database migration 8 adds a `revisions` table and the `extension` and
  `created_at` columns. Every new paste stores its first version in the same
  transaction; the migration writes the first version of every existing paste
  from its stored bytes, without decrypting it or consuming a
  burn-after-reading paste. A paste's versions are deleted with it. The
  database file grows by about the size of the stored pastes, and an older
  executable refuses the migrated database, so a rollback restores the backup.
- The `/v1` JSON API, first operations: `GET /v1`, `POST /v1/pastes`
  (`createPaste`), `GET /v1/pastes` (`listPastes`: the caller's unexpired
  pastes, newest first, paged by `marker`), `GET /v1/pastes/{id}`,
  `DELETE /v1/pastes/{id}`, `GET /v1/pastes/{id}/content` (plain pastes;
  encrypted and burn-after-reading pastes answer 409 and are never consumed)
  and `POST /v1/pastes/{id}/content` (`readContent`: the password and the burn
  confirmation in the body; a burn-after-reading paste is consumed once, after
  a correct password and `confirm_burn`). Every `/v1`
  response carries `X-Request-Id` and `Cache-Control: no-store`, and every
  `/v1` error, the framework's included, uses one JSON shape; a request that
  outlives `WASTEBIN_HTTP_TIMEOUT` is 408 with type `timeout`.
  `docs/api/openapi.yaml` is now generated from the handlers.
- Password key derivation is admission controlled. Every request that derives
  a key (an encrypted create on `/v1` or the legacy routes, a legacy read with
  a password) takes one of `WASTEBIN_KDF_CONCURRENCY` slots (default 2, about
  128 MiB of Argon2 memory). A request that finds no free slot within 1 s is
  429 with `Retry-After: 1`, and nothing is derived, written or consumed.
- Each client may start `WASTEBIN_KDF_RATE` key derivations per minute, with a
  burst of the same number (default 10). Past that, `/v1`, the legacy routes
  and the HTML password form answer 429 with `Retry-After` set to the seconds
  until the next derivation; a read of a plain paste never counts. The client
  is the connection's address, or `CF-Connecting-IP` when the connection comes
  from a range in `WASTEBIN_TRUSTED_PROXIES` (comma-separated CIDR ranges or
  addresses, `unix` for a Unix-socket peer; empty by default). An IPv6 client
  counts by its /64, and `X-Forwarded-For` is never read.

### Changed

- `GET /<id>[.<ext>]` serves the new reading page: the web app from `web/`,
  embedded in the executable, with an ordinary paste embedded as JSON, a link
  preview (title and the first lines) and a `<noscript>` copy of the text. An
  encrypted or burn-after-reading paste's page carries none of it and is never
  consumed by opening it. The page's policy admits images from `https:` and
  `data:`. Building the server from source now starts with `pnpm build` in
  `web/`; without it the reading page is a placeholder.
- `/md/<id>` redirects to `/<id>` with 302; the server-rendered Markdown view
  and its forms are removed.
- `?owner=` answers a 302 with `no-store`, and drops a token that fails as
  well. It was a 303.
- The server executable is named `zpaste`, and log lines name `zpaste` as their
  target. Configuration variables keep the `WASTEBIN_` prefix.
- HTML pages name Zpaste in `<meta name="generator">`.
- The container image runs as uid and gid 10001.
- CI builds the image on every pull request and pushes it to
  `ghcr.io/zakkaus/zpaste` from main (`main` and `sha-<short>`). A version tag
  is published by `release.yml` alone: it checks the tag against the Cargo.toml
  version and main, pushes `<version>` and `sha-<short>`, attaches the static
  x86_64 musl executables copied out of that same image, and writes the image
  digest into the release notes.
- Images are amd64 only and there is no `latest` tag; deployments pin the digest.
  The Dockerfile's arm64 stage builds only with
  `--build-arg CARGO_TARGETS=aarch64-unknown-linux-musl`, and no workflow builds it.
- The image builder is pinned: the `rust:1.95` base by digest, zig by its
  published sha256, `cargo-zigbuild` by version.
- The upstream theme showcase is no longer built; `crates/wastebin_theme_showcase`
  stays in the workspace without a workflow.
