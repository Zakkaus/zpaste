# Zpaste API contract (draft)

[openapi.yaml](openapi.yaml) is the first draft of the `/v1` contract, written
by hand for review (plan section 六, P0). It covers every row of the endpoint
table, the edit and version rules, and the workspace API in
[plan section 四](../plan.zh-TW.md). No handler implements it yet.

Each operation carries `x-phase` (`P1`, `P2` or `P2b`), the phase that builds
it. Operations with `security: []` take no credentials; `createPaste` accepts
none or one, and the rest require an owner token or the web cookie.

Lint it with the pinned Redocly CLI:

```sh
npx --yes @redocly/cli@2.72.1 lint docs/api/openapi.yaml
```

On 2026-11-15 this reported 0 errors and 2 warnings: `GET /` and `GET /v1`
have no 4xx response, because neither has an error state.

## Map to the plan

| Plan | openapi.yaml |
| --- | --- |
| 二, credential and CSRF rules | `securitySchemes` (`ownerToken`, `ownerCookie`), the `Forbidden` response |
| 四, 身分 | `createPaste`, `PasteCreated.owner_token` |
| 四, 端點表 | `paths`: one operation per row; `GET /` is the HTML new paste page |
| 四, 欄位與格式 | `PasteCreate`, `Paste`, `PasteListDocument`, `Link`, `Error`; the `X-Request-Id`, `X-Paste-Password` and `Retry-After` headers |
| 四, 修改與版本 | `updatePaste`, `replaceContent`, `If-Match`, the `PreconditionFailed` and `BurnNotEditable` responses, `Version`, `VersionId` |
| 四, Workspace | `/v1/workspaces` paths, `Workspace*` schemas |
| 四, 舊路由與契約 | Not in the file; see "Generating the contract in P1" |
| 六, phases | `x-phase` on each operation |

Legacy routes (`POST /`, `/raw/{id}`, `DELETE /{id}`) and the HTML reading
routes stay outside the contract. The plan tests the legacy routes by running
the same scenarios against them and `/v1`.

## Decisions made in this draft

The plan does not settle these. Each takes the option closest to the plan; a
reviewer can overturn any of them.

1. Every 401 carries `WWW-Authenticate`, as RFC 9110 requires, not only the
   wrong-password 401 the plan names. Token failures send a Bearer challenge;
   password failures send a separate challenge.
2. A missing or wrong `X-Paste-Password` on `PATCH` or `PUT` is 401, following
   the plan's edit flow, not 409 `password_required`.
3. `PATCH /v1/pastes/{id}` and `PUT .../content` answer 200 with
   `{"paste": {...}}`; the plan names only the status and `ETag`.
4. `GET /v1/pastes/{id}` also sends `ETag`, equal to `version_id` in the body,
   so `gzpaste set` can send `If-Match` without reading the content.
5. For a paste that is both encrypted and burn-after-reading,
   `GET .../content` answers 409 `password_required`, and `POST .../content`
   needs `password` and `confirm_burn` in the same request.
6. `limit` outside 1 to 100 is 400, not clamped. A `marker` naming an expired
   paste gets the same 400 as a deleted one.
7. The owner list applies the same whitelist as `GET /v1/pastes/{id}`: no
   title for an encrypted paste.
8. Body parse errors are 400, not axum's default 422. Every other status uses
   the same error shape, declared once as each operation's `default` response.
9. `expires_at` is null for a paste that never expires (upstream's expiration
   `0`); `expires_in: 0` is valid only where `WASTEBIN_PASTE_EXPIRATIONS`
   lists it. `password` is null or non-empty.
10. `owner_token` is left out of a create that sent valid credentials.
11. Versions are listed newest first and not paged (at most 20). A version has
    `version`, `version_id`, `parent_id`, `created_at` (null for the
    backfilled first version) and, unless encrypted, `title`.
12. The `ETag` of `POST .../versions/{vid}/content` is that version's id.
13. Workspace entry headers are `X-Paste-Extension` and `X-Entry-Kind`; the
    plan says only that headers carry `extension` and `kind`.
    `X-Paste-Extension` is ignored when the entry exists.
14. A `PUT .../entries/{name}` that creates answers 201 with
    `Location: /v1/pastes/{id}` and `{"entry": {...}}`.
15. A taken slug is 409 `slug_taken`.
16. The error `type` values beyond the plan's three (`version_conflict`,
    `password_required`, `confirmation_required`) are listed on `Error.type`
    as a draft vocabulary.
17. The cookie scheme is declared as `ownerCookie`, cookie `uid`, the name
    upstream uses today.
18. `PATCH` lists 429 because editing an encrypted paste derives a key.

## Open questions

These need an owner ruling or a later design step; the draft does not answer
them.

1. The `WWW-Authenticate` scheme name for a paste password.
2. For an encrypted burn paste, does a `POST .../content` with only the
   password verify it (401 or 409 `confirmation_required`) before the
   confirmation step? This decides whether the page can report a wrong
   password before the user confirms.
3. Does `POST .../versions/{vid}/content` with `confirm_burn` consume a burn
   paste, or do the version endpoints refuse burn pastes?
4. Should a plain paste's stored versions be readable by `GET`? The plan has
   only `POST .../versions/{vid}/content`.
5. The fields of a version item, and whether the `revisions` table stores a
   creation time.
6. Should the owner list show the titles of the caller's own encrypted pastes?
7. Does an anonymous create from the web app set the `uid` cookie, and does
   its response still carry `owner_token`?
8. When `PUT .../entries/{name}` creates a paste, how are expiry, password,
   title and burn-after-reading set? Does `X-Paste-Password` on create
   encrypt the new paste?
9. Does `PUT .../entries/{name}` on an existing entry change its `kind`?
10. Entry name rules: character set, length, and whether `/` is allowed.
11. An entry whose paste has expired: does `PUT` create a new paste under the
    same name, and is the entry hidden from `GET`?
12. How is an existing paste added to a workspace? The plan requires it to
    belong to the same identity but names no endpoint for it.
13. Workspace title rules (required, length), a cap on entries, and entry
    order in `GET /v1/workspaces/{slug}`.
14. When a paste's history would exceed its cap (architecture A10), is the
    write refused with 413 or are older versions dropped? The draft lists
    413.
15. `If-Match: *`, weak tags and lists of tags: accept, ignore or 400?
16. Is `POST /v1/pastes` rate limited only when it carries a password (a key
    derivation), or always? The endpoint table lists 429 for every create.
17. Pinned versions of utoipa, openapi-typescript and openapi-fetch
    (architecture A1).

## Generating the contract in P1

From P1 the Rust handlers are the source and this hand-written file is
replaced. The plan (section 四, 舊路由與契約) fixes the pipeline; the steps
below are the proposal for the switch.

1. Annotate each `/v1` handler with `#[utoipa::path]` and each body type with
   `ToSchema`, using the operation ids, schema names and `x-phase` values of
   this draft. Whether the chosen utoipa version emits operation extensions
   is not verified yet.
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
