# Zpaste API contract (draft)

[openapi.yaml](openapi.yaml) is the reviewed draft of the `/v1` contract,
written by hand (plan section 六, P0). It covers every row of the endpoint
table, the edit and version rules, and the workspace API in
[plan section 四](../plan.zh-TW.md). No handler implements it yet.

Each operation carries `x-phase` (`P1`, `P2` or `P2b`), the phase that builds
it. Operations with `security: []` need no credentials; `createPaste` accepts
none or one, and the rest require an owner token or the web cookie. Response
bodies are closed (`additionalProperties: false`), so a leaked field fails
validation.

Lint it with the pinned Redocly CLI:

```sh
npx --yes @redocly/cli@2.72.1 lint docs/api/openapi.yaml
```

On 2026-11-15 this reported 0 errors and 2 warnings: `GET /` and `GET /v1`
have no 4xx response, because neither has an error state.

## Map to the plan

| Plan | openapi.yaml |
| --- | --- |
| 二, credential and CSRF rules | `info.description` (credential selection), `securitySchemes` (`ownerToken`, `ownerCookie`), the `Forbidden` response |
| 四, 身分 | `createPaste`, `PasteCreated.owner_token`, the `Set-Cookie` header of its 201 |
| 四, 端點表 | `paths`: one operation per row; `GET /` is the HTML new paste page |
| 四, 欄位與格式 | `PasteCreate`, `Paste` (`PlainPaste`, `EncryptedPaste`), `PasteListDocument`, `Link`, `Error`; the `X-Request-Id`, `X-Paste-Password` and `Retry-After` headers |
| 四, 修改與版本 | `updatePaste`, `replaceContent`, `If-Match`, the `PreconditionFailed`, `BurnNotEditable` and `BurnNoHistory` responses, `Version`, `VersionId` |
| 四, Workspace | `/v1/workspaces` paths, `Workspace*` schemas, `addWorkspaceEntry` |
| 四, 舊路由與契約 | Not in the file; see "Generating the contract in P1" |
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
17. Pinned in P0; see "Generating the contract in P1".

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

## Generating the contract in P1

From P1 the Rust handlers are the source and this hand-written file is
replaced. The plan (section 四, 舊路由與契約) fixes the pipeline.

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

What the check ran, with Rust 1.99.0:

1. A crate on `utoipa` 6.0.0 produced a document with `x-phase` from
   `#[utoipa::path(extensions(("x-phase" = json!("P1"))))]`, closed schemas
   from `#[serde(deny_unknown_fields)]` (`additionalProperties: false`), a
   `oneOf` from an untagged enum, single-value enums for constants,
   `security: [{}, ownerToken, ownerCookie]` from `security((), ...)`, and
   shared `components.headers`, `components.parameters` and
   `components.securitySchemes` added by a `Modify` implementation.
2. `openapi-typescript` 7.13.0 turned that document, and this draft, into
   types. `tsc` 5.9.3 with `strict` and `exactOptionalPropertyTypes` passed
   two `openapi-fetch` 0.17.0 clients: one calls `GET` and `PATCH` on the
   generated document; the other calls `GET /v1/pastes/{id}`,
   `POST .../content`, `POST .../versions/{vid}/content` and
   `POST .../entries` on this draft and narrows `Paste` by `encrypted`.
   `@ts-expect-error` lines confirmed that `EncryptedPaste` rejects `title`
   in both, and that `VersionReadRequest` rejects `confirm_burn`.

### What utoipa 6.0.0 cannot express, and how the generator handles it

The generator writes every fix in Rust. Nobody edits the generated YAML.

| Gap | Handling |
| --- | --- |
| No `const` in the schema model | A single-value `enum`: a unit enum for strings (`no-store`, `v1`), a small `PartialSchema` implementation for `true` and `false`. This draft already uses that form. |
| `Header` has no `required` field | A serialization-stage patch: the generator turns the typed document into a `serde_json::Value`, sets `required: true` on the shared headers, and writes YAML from that value. |
| `security()` in the path macro emits no `security` key | The `Modify` step sets `security: []` on operations that take no credentials. |
| The path macro cannot reference shared parameters and headers by `$ref` | The `Modify` step inserts `$ref` entries to `components.parameters` and `components.headers` on each operation. |
| Unsigned integers get `format: int32` and `minimum: 0` | `#[schema(minimum = 1)]` where the contract says 1 (checked: it replaces the minimum and keeps the format). |
| Keys come out sorted, not in this draft's order | The first P1 comparison is semantic (both documents parsed and compared), not by bytes; from then on the byte comparison runs against the generated file. |

Behaviour that no schema states (the `Origin` rule, atomic burn consumption,
authorisation, rate limits, `ETag` matching the body) is covered by the
scenario and contract tests, not by the document.

### Operations not built yet

P1 builds only the `P1` operations, but the generated document keeps the whole
contract. Each `P2` and `P2b` operation is declared in a contract-only module
(for example `contract::pending`): a bodiless function carrying
`#[utoipa::path]` and its `x-phase`, which no router mounts and which never
answers a request. A test compares the router's routes with the document: a
route that the document lacks fails it, and so does a pending declaration that
is also routed. Building an operation moves its annotation from the pending
module to the handler in the same pull request.

### Steps

1. Annotate each `/v1` handler with `#[utoipa::path]` and each body type with
   `ToSchema`, using the operation ids, schema names and `x-phase` values of
   this draft. Response types are separate closed structs, one per shape in
   this file (`PlainPaste`, `EncryptedPaste`, `PlainPasteCreated` and so on),
   never the storage struct.
2. Add a command that writes the generated document to
   `docs/api/openapi.yaml`, and a merge-gate step that regenerates it and the
   frontend types and fails on any byte difference.
3. In the first P1 pull request, compare the generated document with this
   reviewed draft. Each difference either changes a handler or is recorded
   and approved in that pull request; then the generated file overwrites this
   one.
4. Contract tests start the server in-process, call every row implemented so
   far with each of its error statuses, and validate status, headers and body
   against the generated file.
