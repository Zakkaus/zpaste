# Zpaste API contract

[openapi.yaml](openapi.yaml) is the `/v1` contract. It covers every row of the
endpoint table, the edit and version rules, and the workspace API in
[plan section 四](../plan.zh-TW.md). It was written by hand and reviewed in P0;
since P1 it is generated from the Rust handlers and committed. Do not edit it
by hand: change the annotations in `crates/wastebin_server/src/v1` and
regenerate it ("Generating the contract").

Each operation carries `x-phase` (`P1`, `P2` or `P2b`), the phase that builds
it. Operations with `security: []` need no credentials; `createPaste` accepts
none or one, and the rest require an owner token or the web cookie. Response
bodies are closed (`additionalProperties: false`), so a leaked field fails
validation.

Lint it with the pinned Redocly CLI:

```sh
npx --yes @redocly/cli@2.72.1 lint docs/api/openapi.yaml
```

On 2026-11-15 this reported 0 errors and 2 warnings, for the hand-written
draft and again for the first generated file: `GET /` and `GET /v1` have no
4xx response, because neither has an error state.

## Map to the plan

| Plan | openapi.yaml |
| --- | --- |
| 二, credential and CSRF rules | `info.description` (credential selection), `securitySchemes` (`ownerToken`, `ownerCookie`), the `Forbidden` response |
| 四, 身分 | `createPaste`, `PasteCreated.owner_token`, the `Set-Cookie` header of its 201 |
| 四, 端點表 | `paths`: one operation per row; `GET /` is the HTML new paste page |
| 四, 欄位與格式 | `PasteCreate`, `Paste` (`PlainPaste`, `EncryptedPaste`), `PasteListDocument`, `Link`, `Error`; the `X-Request-Id`, `X-Paste-Password` and `Retry-After` headers |
| 四, 修改與版本 | `updatePaste`, `replaceContent`, `If-Match`, the `PreconditionFailed`, `BurnNotEditable` and `BurnNoHistory` responses, `Version`, `VersionId` |
| 四, Workspace | `/v1/workspaces` paths, `Workspace*` schemas, `addWorkspaceEntry` |
| 四, 舊路由與契約 | Not in the file; see "Generating the contract" |
| 六, phases | `x-phase` on each operation |

Legacy routes (`POST /`, `/raw/{id}`, `DELETE /{id}`) and the HTML reading
routes stay outside the contract. The plan tests the legacy routes by running
the same scenarios against them and `/v1`.

## Credentials

The full rules are in `info.description` of the file. In short:

- `Authorization` present: only the Bearer token counts, the cookie is ignored,
  and the two never add up to more authority. A token that fails verification
  is 401, with no fallback to the cookie or to an anonymous request.
- No `Authorization`: the `uid` cookie counts. A cookie whose signature fails
  counts as absent, as upstream's `SignedCookieJar` does.
- A request that carries the cookie and writes needs `Origin` equal to the
  public URL, or it is 403 before anything else. `POST .../content` counts as a
  write because it can consume a burn-after-reading paste, so it lists 403
  although it needs no credentials.
- An operation on two resources compares their uids. Adding a paste to a
  workspace requires `paste.uid == workspace.uid`, and a paste created through
  `PUT .../entries/{name}` takes the workspace's uid. Holding both identities
  in one cookie is not enough.
- Credential and `Origin` failures are answered before any key derivation.

A create without credentials hands the new identity over in one of two ways.
A request whose `Origin` equals the public URL is a browser create: the server
sets the `uid` cookie (`HttpOnly; Secure; SameSite=Lax; Path=/`, upstream's
attributes) and the body has no `owner_token`. Any other create gets
`owner_token` in the body, once. A create with a valid Bearer token or cookie
gets neither. A non-browser client that copies the `Origin` gets a cookie
instead of a token, which harms only itself.

## Burn-after-reading and history

- Only `POST /v1/pastes/{id}/content` consumes a burn-after-reading paste, and
  only with `confirm_burn: true` (plus the right `password` when encrypted).
  The consumption is atomic: of two concurrent confirmed requests, one gets the
  content and the other 404.
- For an encrypted burn paste, a request with the right password and no
  `confirm_burn` answers 409 `confirmation_required` and returns or consumes
  nothing; a wrong password is 401. The page can therefore report a wrong
  password before the user confirms. Both requests count against the shared
  derivation limit.
- The history endpoints refuse a burn-after-reading paste with 409
  `burn_no_history`, so there is no second consumption path. They check that
  the paste exists and has not expired (404) before anything else, and find a
  version by the pair `(paste id, version id)`.
- P1 cannot reuse upstream's `Database::get()`
  (`crates/wastebin_core/src/db.rs`, lines 648 to 688) for the confirmation
  step: it deletes a burn paste as soon as the decryption succeeds. P1 must
  split password verification, which consumes nothing, from the atomic
  consumption.

## History limits

Two limits apply after each write, and trimming removes the oldest versions
until both hold. Neither refuses the write, so neither causes 413; 413 means
only that the body exceeds `WASTEBIN_MAX_BODY_SIZE`.

1. At most 20 versions, the current one included (plan section 四).
2. A total size limit on one paste's history (architecture A10, P2). Its
   multiple, and whether it may leave fewer than 20 versions, are pending
   owner rulings below.

## Error types by operation

`Error.type` is a closed list. Every operation also has a `default` response
for statuses no handler chooses: 405 `method_not_allowed`, 415
`unsupported_media_type`, 500 `internal_error`, 503 `service_unavailable`.

| Operation | Status and `type` |
| --- | --- |
| `getVersion` | none |
| `showPaste`, `showWorkspace` | 404 `not_found` |
| `createPaste` | 400 `bad_request`; 401 `unauthorized`; 403 `forbidden`; 413 `payload_too_large`; 429 `rate_limited` |
| `listPastes` | 400 `bad_request`; 401 `unauthorized` |
| `updatePaste` | 400 `bad_request`; 401 `unauthorized` or `wrong_password`; 403 `forbidden`; 404 `not_found`; 409 `burn_not_editable`; 412 `version_conflict`; 429 `rate_limited` |
| `replaceContent` | as `updatePaste`, plus 413 `payload_too_large` |
| `deletePaste`, `deleteWorkspace`, `deleteWorkspaceEntry` | 401 `unauthorized`; 403 `forbidden`; 404 `not_found` |
| `getContent` | 404 `not_found`; 409 `password_required` or `confirmation_required` |
| `readContent` | 400 `bad_request`; 401 `wrong_password`; 403 `forbidden`; 404 `not_found`; 409 `password_required` or `confirmation_required`; 429 `rate_limited` |
| `listVersions` | 404 `not_found`; 409 `burn_no_history` |
| `readVersionContent` | 400 `bad_request`; 401 `wrong_password`; 404 `not_found`; 409 `burn_no_history` or `password_required`; 429 `rate_limited` |
| `createWorkspace` | 400 `bad_request`; 401 `unauthorized`; 403 `forbidden`; 409 `slug_taken` |
| `updateWorkspace` | 400 `bad_request`; 401 `unauthorized`; 403 `forbidden`; 404 `not_found` |
| `addWorkspaceEntry` | 400 `bad_request`; 401 `unauthorized`; 403 `forbidden`; 404 `not_found`; 409 `entry_name_taken`, `paste_in_workspace` or `workspace_full` |
| `putWorkspaceEntry` | 400 `bad_request`; 401 `unauthorized` or `wrong_password`; 403 `forbidden`; 404 `not_found`; 409 `burn_not_editable` or `workspace_full`; 412 `version_conflict`; 413 `payload_too_large`; 429 `rate_limited` |

## Decisions made in this draft

The plan does not settle these. The review of 2026-11-15 accepted D1 to D18;
the text below includes the adjustments it asked for. D19 to D24 were added
while applying the review, and a reviewer can overturn any of them.

1. Every 401 carries `WWW-Authenticate`, as RFC 9110 requires, not only the
   wrong-password 401 the plan names. Token failures send
   `Bearer realm="zpaste"`; password failures send
   `Paste-Password realm="zpaste"`.
2. A missing or wrong `X-Paste-Password` on `PATCH` or `PUT` is 401, following
   the plan's edit flow, not 409 `password_required`.
3. `PATCH /v1/pastes/{id}` and `PUT .../content` answer 200 with
   `{"paste": {...}}`; the plan names only the status and `ETag`.
4. `GET /v1/pastes/{id}` also sends `ETag`, equal to `version_id` in the body,
   so `gzpaste set` can send `If-Match` without reading the content.
5. For a paste that is both encrypted and burn-after-reading,
   `GET .../content` answers 409 `password_required`, and consuming it needs
   `password` and `confirm_burn` in the same request. The request with only
   the right password answers 409 `confirmation_required` (Q2).
6. `limit` outside 1 to 100 is 400, not clamped. A `marker` naming an expired
   paste gets the same 400 as a deleted one, so it reveals nothing.
7. The owner list applies the same whitelist as `GET /v1/pastes/{id}`: no
   title for an encrypted paste, even the caller's own.
8. Body parse errors are 400, not axum's default 422. Every error uses one
   shape. Each operation lists the statuses it can choose explicitly; the
   `default` response covers only statuses no handler chooses.
9. `expires_at` is null for a paste that never expires (upstream's expiration
   `0`); `expires_in: 0` is valid only where `WASTEBIN_PASTE_EXPIRATIONS`
   lists it. `password` is null or non-empty.
10. `owner_token` is left out of a create that sent valid credentials.
11. Versions are listed newest first and not paged (at most 20). A version has
    `version`, `version_id`, `parent_id`, `created_at` (null for the
    backfilled first version), `encrypted` and, unless encrypted, `title`.
    `revisions` stores the creation time.
12. The `ETag` of `POST .../versions/{vid}/content` is that version's id.
13. Workspace entry headers are `X-Paste-Extension`, `X-Paste-Expires`,
    `X-Paste-Title` and `X-Entry-Kind`; the plan says only that headers carry
    `extension` and `kind`. `X-Paste-Extension` is ignored when the entry
    exists, because an extension never changes.
14. A `PUT .../entries/{name}` that creates, and `POST .../entries`, answer
    201 with `Location: /v1/pastes/{id}` and `{"entry": {...}}`.
15. A taken slug is 409 `slug_taken`.
16. `Error.type` is a closed list; "Error types by operation" maps each
    operation and status to its types.
17. The cookie scheme is declared as `ownerCookie`, cookie `uid`, the name
    upstream uses today.
18. `PATCH` lists 429 because editing an encrypted paste derives a key.
19. With the cookie, `owner=me` lists the pastes of every identity the cookie
    holds, and a single-resource operation is allowed when any of them owns
    the resource.
20. A cookie whose signature fails counts as absent (upstream's behaviour), so
    a browser with a stale cookie can still create; a Bearer token that fails
    is 401.
21. `Version` and `WorkspaceEntry` carry `encrypted`, so each has two closed
    shapes and an encrypted item cannot carry a title.
22. On a plain paste, version or entry, `title` is always present (null when
    there is none), so a client never has to tell a missing field from an
    empty title.
23. A malformed `If-Match` is 400.
24. `X-Request-Id` has no pattern on input, because a non-matching value is
    replaced, not rejected; the response header keeps the pattern.

## Answers to the open questions

The review of 2026-11-15 settled these; the file implements each answer.

1. Password challenge: `WWW-Authenticate: Paste-Password realm="zpaste"`, a
   custom scheme of this API, not registered with IANA.
2. Encrypted burn paste: a `POST .../content` with only the right password
   answers 409 `confirmation_required`, returns nothing and consumes nothing; a
   wrong one is 401. Both steps count against the shared derivation limit.
3. Every history endpoint refuses a burn-after-reading paste with 409
   `burn_no_history`.
4. Historical versions are read only by `POST .../versions/{vid}/content`, the
   plan's endpoint; a plain version takes the body `{}`. There is no `GET`.
5. Version fields as in D11; `revisions` stores `created_at`.
6. No: see D7.
7. Partly: a valid Bearer token never gets a token back (D10). A browser create
   without credentials gets an `HttpOnly; Secure; SameSite=Lax` cookie and no
   `owner_token`; the server recognises it by an `Origin` equal to the public
   URL ("Credentials"). This changes plan text; see "Plan text to amend".
8. A create through `PUT .../entries/{name}` takes `X-Paste-Expires` (one of
   the allowed values, default otherwise) and `X-Paste-Title` (percent-encoded
   UTF-8, at most 256 characters). It cannot encrypt or burn:
   `X-Paste-Password` on a create is 400, so a password is never stored as
   plain text by mistake.
9. An omitted `X-Entry-Kind` keeps the kind. A sent one changes it in the same
   transaction as the content; a 412 or any other failure leaves both.
10. Pending owner ruling, below.
11. An entry whose paste has expired is hidden from `GET`. A `PUT` without
    `If-Match` creates a new paste under the same name; a `PUT` with
    `If-Match` (stale tag or `*`) is 412 and creates nothing.
12. `POST /v1/workspaces/{slug}/entries` with
    `{"entry": {"name", "kind", "paste_id"}}`. The paste's uid must equal the
    workspace's (403 otherwise). A taken name, a paste already in a workspace
    and a full workspace are 409.
13. Entry order: `kind` ascending, entries without a kind last; then
    `updated_at` newest first; then `name`. A full workspace is 409
    `workspace_full`, not 413. Title rules and the entry limit are pending
    owner rulings, below.
14. Replaced: see "History limits" and the A10 ruling below.
15. `If-Match` takes RFC 9110 syntax. `*` matches any current version, a weak
    tag never matches, and a list matches when any strong tag equals the
    current one.
16. Only paths that derive a key are rate limited by the server: reading with
    a password (current or historical, the confirmation step included),
    creating an encrypted paste, editing an encrypted paste, and the legacy
    routes. The general create rate stays at the proxy (Traefik, average 8,
    burst 40). Credential failures are answered before any derivation.
17. Pinned in P0; see "Generating the contract".

## Pending owner rulings

The file carries the recommended default for each, so it stays consistent;
the owner may choose the alternative.

| Item | Recommended (in the file) | Alternative | Where in the file |
| --- | --- | --- | --- |
| Q10, entry name | Any Unicode except `/` and control characters; not `.` or `..`; NFC-normalised; 1 to 128 bytes of UTF-8. Chinese file names work. | ASCII `[A-Za-z0-9._-]{1,64}`, which rejects Chinese names. | `EntryName`, `EntryNamePath` |
| Q13, workspace title | Optional; defaults to the slug; 1 to 200 characters. | Required at create. | `WorkspaceTitle`, `createWorkspace` |
| Q13, entries per workspace | At most 500. | Another number, or no limit. | `Workspace.entries.maxItems`, `workspace_full` |
| A10, history total | At most 4 × `WASTEBIN_MAX_BODY_SIZE` per paste; trim the oldest versions until both limits hold, which may keep fewer than 20. | Keep the 20-version promise and refuse a write that would exceed the total with 413. | `replaceContent`, "History limits" |

## Plan text to amend

The answers change these lines of [the plan](../plan.zh-TW.md); the plan owner
amends them.

- Lines 109, 111 and 156 say the first create returns `owner_token` and the
  web app keeps the token. After Q7, a browser create gets an `HttpOnly`
  cookie and no `owner_token`; only a non-browser create gets the token.
- The workspace API list (line 242 and the lines around it) lacks
  `POST /v1/workspaces/{slug}/entries` (Q12).
- Lines 191 and 211 give the 20-version limit and the history total
  separately; after the A10 ruling, state how the two interact.


## Generating the contract

The Rust handlers are the source of the contract (plan section 四,
舊路由與契約). `crates/wastebin_server/src/v1/contract` assembles the
document in the tests; nothing in the release executable builds it.

Regenerate the file after changing an annotation:

```sh
cargo test -p wastebin -- --ignored write_openapi_document
```

`cargo test --all-targets` in the merge gate runs these checks:

| Test | Fails when |
| --- | --- |
| `v1::contract::tests::document_matches_committed` | `openapi.yaml`, parsed, differs from the generated document; it lists the JSON pointers that differ. Key order and YAML style do not count. |
| `v1::contract::tests::router_matches_document` | a routed `/v1` operation has no annotation, an annotated handler is not routed, a declaration in `contract::pending` is routed, or a routed operation's `x-phase` is not `P1` |
| `v1::tests` | a built operation answers a status, header or body that its entry in the generated document does not allow; one test per operation and status |
| `v1::contract::schema::tests` | the validator of the contract tests accepts a planted violation (at the top level, in a `oneOf` branch, in an optional property, or of a `format`), or compiles a malformed schema |

### Pinned versions

Checked on 2026-11-15 in a scratch crate and a scratch TypeScript project
(not committed). Each version was released at least 14 days earlier.

| Tool | Version | Released | Role |
| --- | --- | --- | --- |
| `utoipa` | 6.0.0 | 2026-09-22 | Annotations and the OpenAPI model |
| `utoipa-gen` | 6.0.1 (pulled by `utoipa` 6.0.0) | 2026-09-23 | The derive macros |
| `yaml_serde` | 0.10.7 (also `utoipa`'s `yaml` feature) | 2026-08-18 | Writes the patched document as YAML |
| `openapi-typescript` | 7.13.0 | 2026-02-11 | Frontend types |
| `openapi-fetch` | 0.17.0 | 2026-02-11 | Typed client |
| `typescript` | 5.9.3 | 2025-09-30 | `openapi-typescript` 7.13.0 requires `^5.x` |
| `@redocly/cli` | 2.72.1 | | Lint, already pinned |
| `jsonschema` | 0.60.2 (`default-features = false`) | 2026-10-26 | Validates responses against the generated document in the tests: draft 2020-12, `format` asserted, unknown formats refused |

In P0 a crate on `utoipa` 6.0.0 produced `x-phase`, closed schemas, `oneOf`
from untagged enums and `security: [{}, ownerToken, ownerCookie]`;
`openapi-typescript` 7.13.0 and `openapi-fetch` 0.17.0 type-checked clients of
that document and of the draft with `tsc` 5.9.3 (`strict`,
`exactOptionalPropertyTypes`).

### Where each part comes from

- Operations: `#[utoipa::path]` on the handlers in `v1/pastes.rs` (and
  `GET /` on `handlers/html/index.rs`), compiled only in tests
  (`#[cfg_attr(test, utoipa::path(...))]`). Summary and description come from
  the handler's doc comment.
- Schemas: `ToSchema` on the request and response types in `v1/model.rs`.
  Each response shape is its own closed struct (`#[serde(deny_unknown_fields)]`
  gives `additionalProperties: false`), never the storage struct.
- Shared components (`securitySchemes`, `parameters`, `headers`,
  `responses`) and the `$ref`s that point to them: the `Contract` modifier in
  `v1/contract/components.rs`. The error responses are marker types in
  `v1/contract/responses.rs`, so a handler names them in `responses(...)`.
- Operations not built yet: `v1/contract/pending` (below).

### What utoipa 6.0.0 cannot express, and how the generator handles it

The generator writes every fix in Rust. Nobody edits the generated YAML.

| Gap | Handling |
| --- | --- |
| No `const` in the schema model | A single-value `enum`: a unit enum for strings (`no-store`, `v1`), a small `PartialSchema` implementation for `true` and `false` (`model::True`, `model::False`). |
| `Header` has no `required` field | `Contract` builds every header with an `x-required` extension; the serialisation patch renames it to `required`. |
| `security(())` is written as `security: [{}]` | The serialisation patch writes `security: []`. |
| The path macro cannot reference shared parameters and headers by `$ref` | `Contract` inserts them from tables keyed by path placeholder and operation id, and adds `X-Request-Id`, `Cache-Control` and the `default` response to every `/v1` operation. |
| `info(...)` has no `summary`, and `Info` has no such field | The serialisation patch sets `info.summary`. |
| No `uri` among the known formats | `format = "uri"`, a custom format with the same output. |
| Unsigned integers get `format: int32` or `int64`, and `minimum: 0` | Kept; `#[schema(minimum = 1)]` where the contract says 1. Recorded below. |

Behaviour that no schema states (the `Origin` rule, atomic burn consumption,
authorisation, rate limits, `ETag` matching the body) is covered by the
contract tests, not by the document.

### Differences from the reviewed draft

The first generated file was compared with the reviewed draft (commit
`3fa7a85`) as parsed structures, with strings right-trimmed, path-item
parameters moved onto their operations, and `required` and `parameters`
lists sorted. These differences remain, and the generated file replaced the
draft:

1. `info.version` is `0.1.0`, not `0.1.0-draft`, and the first paragraph of
   `info.description` says the file is generated.
2. Integer properties and parameters carry `format: int32` or `int64` from
   their Rust types; `Error.code` also gets `minimum: 0`.
3. Shared path parameters are listed on each operation instead of the path
   item, and an operation's parameters come in a different order. The
   meaning is the same.
4. The description of `parent_id` (P2) sits next to the `$ref` inside its
   `oneOf` instead of next to the `oneOf`.
5. Operation descriptions lose the final newline that YAML block scalars add.

`yaml_serde` writes timestamp-like strings such as `2026-11-21T13:00:00Z`
(two examples) without quotes. They are strings in YAML 1.2, which the
generator and the tests use; a YAML 1.1 parser (PyYAML, for one) reads them as
timestamps.

### Operations not built yet

The generated document keeps the whole contract. Each operation not built
yet is declared in `v1/contract/pending/mod.rs`: a bodiless function carrying
`#[utoipa::path]` and its `x-phase`, which no router mounts and which never
answers a request. In this slice that is every `P2` and `P2b` operation plus
`listPastes` and `readContent` (`P1`, next slice). Building an operation
moves its annotation from the pending module to the handler and its route
into `v1_routes!` in `v1/mod.rs`, in the same pull request;
`router_matches_document` fails otherwise. Schemas only those operations
use are in `v1/contract/pending/schemas.rs`.

## Behaviour of the built operations

Decided while building P1, within the contract:

- An `Origin` check applies to a writing request that carries a `uid` cookie
  and no `Authorization`, whether or not the cookie's signature verifies; it
  runs before the signature check. A Bearer request skips it.
- A signed cookie that holds no uid counts as absent.
- `expires_in` omitted or null takes the configured value marked `=d`, or the
  first configured value when none is marked.
- A path id that does not parse (wrong length or alphabet, or with an
  extension) is 404, like a missing paste.
- `DELETE /v1/pastes/{id}` checks credentials and `Origin` (401, 403), then
  that the paste exists and has not expired (404), then ownership (403). A
  paste without an owner, from before upstream stored uids, is 403 for
  everyone.
- `X-Request-Id` the server makes is 32 lowercase hex digits.
- A timeout (408 from the server's HTTP timeout) keeps upstream's empty body:
  `Error.type` has no value for it. Adding one changes the closed list and
  waits for the owner.
