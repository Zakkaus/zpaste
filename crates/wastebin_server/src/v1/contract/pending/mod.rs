//! Operations of the contract that no router mounts yet, with the schemas only they use.
//!
//! Each function is a bodiless declaration that carries its `#[utoipa::path]` and `x-phase`. It
//! never answers a request. Building an operation moves its annotation to the handler in the same
//! pull request; `contract::tests::router_matches_document` fails on a declaration that is also
//! routed.

#![allow(
    dead_code,
    reason = "declarations for the generated contract; nothing calls them"
)]

use schemas::{
    PasteUpdateRequest, VersionListDocument, VersionReadRequest, WorkspaceCreateRequest,
    WorkspaceDocument, WorkspaceEntryAddRequest, WorkspaceEntryDocument, WorkspaceUpdateRequest,
};

use super::responses::{
    BadRequest, BurnNoHistory, BurnNotEditable, EntryAddConflict, EntryPreconditionFailed,
    EntryWriteConflict, Forbidden, NotFound, PayloadTooLarge, PreconditionFailed, SlugTaken,
    TooManyRequests, Unauthorized, VersionReadConflict, WrongPassword,
};
use crate::v1::model::{ContentDocument, Extension, VersionId};

pub(crate) mod schemas;

/// Change the title
///
/// Follows the edit rules of plan section 四, in this order: credentials and
/// `Origin` (401, 403); burn-after-reading is 409; an encrypted paste needs
/// `X-Paste-Password`, and a password that cannot decrypt the current content
/// is 401 and writes nothing. Then, in one transaction: an `If-Match` that
/// matches no current version is 412; a title equal to the current one is 200
/// with no new version; otherwise a new version is written. Expiry does not
/// change. 429 applies only to an encrypted paste, which derives a key.
#[utoipa::path(
    patch,
    path = "/v1/pastes/{id}",
    tag = "pastes",
    operation_id = "updatePaste",
    extensions(("x-phase" = json!("P2"))),
    security(("ownerToken" = []), ("ownerCookie" = [])),
    request_body(content = PasteUpdateRequest, example = json!({"paste": {"title": "Plan v2"}})),
    responses(
        (status = 200, description = "Updated, or unchanged when the title already matched.",
            body = crate::v1::model::PasteDocument),
        (status = 400, response = BadRequest),
        (status = 401, response = Unauthorized),
        (status = 403, response = Forbidden),
        (status = 404, response = NotFound),
        (status = 409, response = BurnNotEditable),
        (status = 412, response = PreconditionFailed),
        (status = 429, response = TooManyRequests),
    ),
)]
fn update_paste() {}

/// Replace the content
///
/// The body is the raw new text. Same edit rules and order as
/// `PATCH /v1/pastes/{id}`: burn-after-reading is 409; an encrypted paste
/// needs `X-Paste-Password` and is re-encrypted with the same password and a
/// new nonce; an `If-Match` that matches no current version is 412; content
/// and title equal to the current version give 200 with no new version;
/// otherwise a new version is written. Expiry does not change.
///
/// After the write, history is trimmed, oldest first, until both limits hold:
/// at most 20 versions, the current one included, and at most
/// 4 × `WASTEBIN_MAX_BODY_SIZE` bytes over all versions of the paste.
/// The size limit may leave fewer than 20 versions. Trimming never refuses the
/// write.
#[utoipa::path(
    put,
    path = "/v1/pastes/{id}/content",
    tag = "content",
    operation_id = "replaceContent",
    extensions(("x-phase" = json!("P2"))),
    security(("ownerToken" = []), ("ownerCookie" = [])),
    request_body(content = String, content_type = "text/plain; charset=utf-8"),
    responses(
        (status = 200, description = "Replaced, or unchanged when the content already matched.",
            body = crate::v1::model::PasteDocument),
        (status = 400, response = BadRequest),
        (status = 401, response = Unauthorized),
        (status = 403, response = Forbidden),
        (status = 404, response = NotFound),
        (status = 409, response = BurnNotEditable),
        (status = 412, response = PreconditionFailed),
        (status = 413, response = PayloadTooLarge),
        (status = 429, response = TooManyRequests),
    ),
)]
fn replace_content() {}

/// List stored versions
///
/// Newest first; at most 20 versions are kept, so there is no paging. A
/// missing or expired paste is 404, checked first. A burn-after-reading paste
/// has no history endpoints: 409 `burn_no_history`.
#[utoipa::path(
    get,
    path = "/v1/pastes/{id}/versions",
    tag = "history",
    operation_id = "listVersions",
    extensions(("x-phase" = json!("P2"))),
    security(()),
    responses(
        (status = 200, description = "The stored versions.", body = VersionListDocument),
        (status = 404, response = NotFound),
        (status = 409, response = BurnNoHistory),
    ),
)]
fn list_versions() {}

/// Read the content of one stored version
///
/// The version is looked up by the pair `(id, vid)`: a `vid` of another
/// paste, or one trimmed from history, is 404. Checks run in this order: the
/// paste exists and has not expired (404); the paste is not
/// burn-after-reading (409 `burn_no_history`; this endpoint never consumes
/// anything); the version exists (404); then the password.
///
/// The body is `{}` for a plain paste and `{"password": ...}` for an
/// encrypted one. A missing password is 409 `password_required`, a wrong one
/// 401; a request with a password counts against the shared derivation limit
/// (429). `POST` serves every version, plain ones included, so that one
/// client call covers both cases.
#[utoipa::path(
    post,
    path = "/v1/pastes/{id}/versions/{vid}/content",
    tag = "history",
    operation_id = "readVersionContent",
    extensions(("x-phase" = json!("P2"))),
    security(()),
    params(("vid" = VersionId, Path, description = "A `version_id` from this paste's version list.")),
    request_body(content = VersionReadRequest, examples(
        ("plain" = (value = json!({}))),
        ("encrypted" = (value = json!({"password": "hunter2"}))),
    )),
    responses(
        (status = 200, description = "The content of that version; `ETag` is that version's id.",
            body = ContentDocument),
        (status = 400, response = BadRequest),
        (status = 401, response = WrongPassword),
        (status = 404, response = NotFound),
        (status = 409, response = VersionReadConflict),
        (status = 429, response = TooManyRequests),
    ),
)]
fn read_version_content() {}

/// Create a workspace
///
/// Slugs are first come, first served. The workspace belongs to the identity
/// that creates it (with the cookie, its first). An omitted title defaults to
/// the slug.
#[utoipa::path(
    post,
    path = "/v1/workspaces",
    tag = "workspaces",
    operation_id = "createWorkspace",
    extensions(("x-phase" = json!("P2b"))),
    security(("ownerToken" = []), ("ownerCookie" = [])),
    request_body(content = WorkspaceCreateRequest,
        example = json!({"workspace": {"slug": "v2raya", "title": "v2rayA triage"}})),
    responses(
        (status = 201, description = "Created.", body = WorkspaceDocument),
        (status = 400, response = BadRequest),
        (status = 401, response = Unauthorized),
        (status = 403, response = Forbidden),
        (status = 409, response = SlugTaken),
    ),
)]
fn create_workspace() {}

/// Show a workspace and its entries
///
/// Anyone with the link can read it (unlisted). Entries of encrypted pastes
/// carry no title. Entries whose paste has expired are left out. Order:
/// `kind` ascending by code point, entries without a kind last; then
/// `updated_at` newest first, unknown times last; then `name` ascending.
#[utoipa::path(
    get,
    path = "/v1/workspaces/{slug}",
    tag = "workspaces",
    operation_id = "showWorkspace",
    extensions(("x-phase" = json!("P2b"))),
    security(()),
    responses(
        (status = 200, description = "The workspace.", body = WorkspaceDocument),
        (status = 404, response = NotFound),
    ),
)]
fn show_workspace() {}

/// Change the title
#[utoipa::path(
    patch,
    path = "/v1/workspaces/{slug}",
    tag = "workspaces",
    operation_id = "updateWorkspace",
    extensions(("x-phase" = json!("P2b"))),
    security(("ownerToken" = []), ("ownerCookie" = [])),
    request_body(content = WorkspaceUpdateRequest),
    responses(
        (status = 200, description = "Updated.", body = WorkspaceDocument),
        (status = 400, response = BadRequest),
        (status = 401, response = Unauthorized),
        (status = 403, response = Forbidden),
        (status = 404, response = NotFound),
    ),
)]
fn update_workspace() {}

/// Delete a workspace
///
/// Deletes the grouping only. Its pastes keep their own expiry and links.
#[utoipa::path(
    delete,
    path = "/v1/workspaces/{slug}",
    tag = "workspaces",
    operation_id = "deleteWorkspace",
    extensions(("x-phase" = json!("P2b"))),
    security(("ownerToken" = []), ("ownerCookie" = [])),
    responses(
        (status = 204, description = "Deleted."),
        (status = 401, response = Unauthorized),
        (status = 403, response = Forbidden),
        (status = 404, response = NotFound),
    ),
)]
fn delete_workspace() {}

/// Add an existing paste under a name
///
/// The paste must belong to the same identity as the workspace: the server
/// compares the two uids, and holding both identities in one cookie is not
/// enough (403). A paste already in a workspace, a taken name and a full
/// workspace are 409. A missing or expired workspace or paste is 404.
#[utoipa::path(
    post,
    path = "/v1/workspaces/{slug}/entries",
    tag = "workspaces",
    operation_id = "addWorkspaceEntry",
    extensions(("x-phase" = json!("P2b"))),
    security(("ownerToken" = []), ("ownerCookie" = [])),
    request_body(content = WorkspaceEntryAddRequest,
        example = json!({"entry": {"name": "plan.md", "kind": "plan", "paste_id": "abc123"}})),
    responses(
        (status = 201, description = "Added.", body = WorkspaceEntryDocument),
        (status = 400, response = BadRequest),
        (status = 401, response = Unauthorized),
        (status = 403, response = Forbidden),
        (status = 404, response = NotFound),
        (status = 409, response = EntryAddConflict),
    ),
)]
fn add_workspace_entry() {}

/// Create or update a named entry
///
/// The body is the raw text. The caller must hold the workspace's identity.
/// The link of the paste never changes while the entry lives.
///
/// When the name does not exist, or its paste has expired, a new paste is
/// created in the workspace's identity and bound to the name (201). Its
/// expiry comes from `X-Paste-Expires` and its title from `X-Paste-Title`.
/// A create cannot encrypt or burn: `X-Paste-Password` on a create is 400
/// (create such a paste with `POST /v1/pastes` and add it). An `If-Match`
/// on a create matches no current version and is 412, so a stale write
/// never recreates an expired entry. A full workspace is 409
/// `workspace_full`.
///
/// When the entry exists, this is `PUT /v1/pastes/{id}/content` on its
/// paste, with the same rules, `If-Match` and `X-Paste-Password` included
/// (200). `X-Paste-Expires` and `X-Paste-Title` are 400 here. When
/// `X-Entry-Kind` is sent, the kind changes in the same transaction as the
/// content; when omitted, the kind stays. A 412 or any other failure leaves
/// both unchanged.
#[utoipa::path(
    put,
    path = "/v1/workspaces/{slug}/entries/{name}",
    tag = "workspaces",
    operation_id = "putWorkspaceEntry",
    extensions(("x-phase" = json!("P2b"))),
    security(("ownerToken" = []), ("ownerCookie" = [])),
    params(
        ("X-Paste-Extension" = Option<Extension>, Header, nullable = false,
            description = "Extension of a new entry's paste. Ignored when the entry exists, because an extension never changes after creation."),
        ("X-Paste-Expires" = Option<u64>, Header, nullable = false, minimum = 0,
            description = "Expiry of a new entry's paste in seconds, with the rules of `PasteCreate.expires_in`. Omitted takes the configured default."),
        ("X-Paste-Title" = Option<String>, Header, nullable = false, min_length = 1,
            description = "Title of a new entry's paste, percent-encoded UTF-8; at most 256 characters once decoded."),
        ("X-Entry-Kind" = Option<String>, Header, nullable = false, min_length = 1,
            description = "Free-text category, for example `plan`, `result`, `review`, `log`."),
    ),
    request_body(content = String, content_type = "text/plain; charset=utf-8"),
    responses(
        (status = 200, body = WorkspaceEntryDocument,
            description = "The entry's paste was updated, or unchanged when the content already matched."),
        (status = 201, description = "A new paste was created and bound to this name.",
            body = WorkspaceEntryDocument),
        (status = 400, response = BadRequest),
        (status = 401, response = Unauthorized),
        (status = 403, response = Forbidden),
        (status = 404, response = NotFound),
        (status = 409, response = EntryWriteConflict),
        (status = 412, response = EntryPreconditionFailed),
        (status = 413, response = PayloadTooLarge),
        (status = 429, response = TooManyRequests),
    ),
)]
fn put_workspace_entry() {}

/// Remove an entry
///
/// Removes the entry from the workspace. The paste itself stays.
#[utoipa::path(
    delete,
    path = "/v1/workspaces/{slug}/entries/{name}",
    tag = "workspaces",
    operation_id = "deleteWorkspaceEntry",
    extensions(("x-phase" = json!("P2b"))),
    security(("ownerToken" = []), ("ownerCookie" = [])),
    responses(
        (status = 204, description = "Removed."),
        (status = 401, response = Unauthorized),
        (status = 403, response = Forbidden),
        (status = 404, response = NotFound),
    ),
)]
fn delete_workspace_entry() {}
