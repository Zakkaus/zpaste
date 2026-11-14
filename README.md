# Zpaste

English | [繁體中文](README.zh-TW.md) | [简体中文](README.zh-CN.md)

Zpaste is a self-hosted pastebin for reading Markdown, code and logs, whether a
person wrote them or an AI tool generated them. It is built to replace the
wastebin instance at https://paste.gentoozh.org and to make long pastes readable
there. Editing a paste at the same URL and keeping its version history are
planned.

Zpaste is based on [wastebin](https://github.com/matze/wastebin) by Matthias
Vogelgesang and keeps wastebin's storage, encryption, expiration and
burn-after-reading code.

## Status

Zpaste is in early development. Its behaviour currently equals upstream
wastebin at commit `c0ec968`, except that the server executable is named
`zpaste` and log lines name `zpaste` as their target. Configuration variables
keep the `WASTEBIN_` prefix, and the HTTP API is wastebin's.

## Build

Building requires Rust 1.95 or later and a C compiler, because SQLite and
zstd are compiled from source. Run this in the repository root:

```sh
cargo build --release
```

The build writes the server to `target/release/zpaste` and the database tool
to `target/release/wastebin-ctl`.

## Run

Start the server with a database file in the current directory:

```sh
WASTEBIN_DATABASE_PATH=state.db ./target/release/zpaste
```

The server listens on `0.0.0.0:8088`. Without `WASTEBIN_DATABASE_PATH` it keeps
pastes in memory and loses them on exit. Open http://127.0.0.1:8088 in a
browser, or create a paste from a shell:

```sh
curl -s -H 'Content-Type: application/json' \
    --data '{"text": "# Hello", "extension": "md"}' \
    http://127.0.0.1:8088/
```

The response names the new paste and an owner token that can delete it:

```json
{"path":"/<id>.md","owner":"<token>"}
```

`curl http://127.0.0.1:8088/raw/<id>` returns the stored text. The
configuration variables, the rest of the HTTP API and `wastebin-ctl` are
described in [wastebin's README](docs/upstream/WASTEBIN-README.md#configuration).

## Container image

The Dockerfile cross-compiles static musl executables and is meant for an
x86_64 host. Build the x86_64 image in the repository root:

```sh
docker build --target amd64 -t zpaste .
```

The image runs `/app/zpaste` as user 10001.

## Test

```sh
cargo test
```

## Planned

- A reading page that renders Markdown with an outline, and shows code, logs
  and diffs in views suited to each.
- Editing a paste at the same URL, with version history.
- A versioned HTTP API under `/v1` and a command-line client.

## Upstream documents

wastebin's README, changelog and contribution guide are kept in
[docs/upstream](docs/upstream/README.md) for reference. They describe wastebin
and will drift from Zpaste.

## License

Zpaste is released under the [MIT license](LICENSE). The license keeps
Matthias Vogelgesang's copyright for wastebin and adds Zakkaus's copyright for
Zpaste.
