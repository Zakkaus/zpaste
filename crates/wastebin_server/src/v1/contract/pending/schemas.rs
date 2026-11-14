//! Schemas that only the operations in [`super`] use.

#![allow(
    dead_code,
    reason = "schemas of the generated contract; nothing builds them yet"
)]

use serde::Serialize;
use utoipa::ToSchema;

use crate::v1::model::{Link, Paste, PasteId, PasteLinks, Title, True, VersionId};

/// A strong entity tag, `"<version_id>"`.
#[derive(Serialize, ToSchema)]
#[schema(pattern = r#"^"[0-9a-f]{64}"$"#)]
pub(crate) struct EntityTag(String);

/// `*`, or entity tags (strong or `W/` weak) separated by commas.
#[derive(Serialize, ToSchema)]
#[schema(
    pattern = r#"^\s*(\*|(W/)?"[^"]*"(\s*,\s*(W/)?"[^"]*")*)\s*$"#,
    example = r#""9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08""#
)]
pub(crate) struct IfMatchList(String);

#[derive(Serialize, ToSchema)]
#[schema(pattern = "^[a-z0-9-]{2,40}$", example = "v2raya")]
pub(crate) struct Slug(String);

/// See the `name` path parameter. The byte limit (128 bytes of UTF-8) is enforced by the server.
#[derive(Serialize, ToSchema)]
#[schema(
    min_length = 1,
    max_length = 128,
    pattern = r"^(?!\.\.?$)[^/\u0000-\u001F\u007F-\u009F]+$",
    example = "plan.md"
)]
pub(crate) struct EntryName(String);

/// 1 to 200 characters.
#[derive(Serialize, ToSchema)]
#[schema(min_length = 1, max_length = 200)]
pub(crate) struct WorkspaceTitle(String);

/// RFC 3339, UTC.
#[derive(Serialize, ToSchema)]
#[schema(format = DateTime, example = "2026-11-21T13:00:00Z")]
pub(crate) struct Timestamp(String);

/// Free-text category; null groups under "other".
#[derive(Serialize, ToSchema)]
#[schema(min_length = 1)]
pub(crate) struct Kind(Option<String>);

#[derive(Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct PasteListDocument {
    pastes: Vec<Paste>,
    /// Holds one `next` link while more pages exist; empty on the last page.
    #[schema(max_items = 1, example = json!([
        {"rel": "next", "href": "https://paste.gentoozh.org/v1/pastes?owner=me&limit=20&marker=abc123"}
    ]))]
    pastes_links: Vec<Link>,
}

#[derive(Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct PasteUpdate {
    #[schema(required = true)]
    title: Title,
}

#[derive(Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct PasteUpdateRequest {
    paste: PasteUpdate,
}

/// `password` for an encrypted paste, `confirm_burn: true` for a
/// burn-after-reading paste, both in one request to consume an encrypted burn
/// paste. See `readContent` for every combination.
#[derive(Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ContentReadRequest {
    #[schema(nullable = false, min_length = 1)]
    password: Option<String>,
    #[schema(inline, nullable = false)]
    confirm_burn: Option<True>,
}

/// `{}` for a plain paste, `{"password": ...}` for an encrypted one. History never consumes, so there is no `confirm_burn`.
#[derive(Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct VersionReadRequest {
    #[schema(nullable = false, min_length = 1)]
    password: Option<String>,
}

#[derive(Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
struct Content {
    text: String,
    version_id: VersionId,
}

#[derive(Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ContentDocument {
    #[schema(inline)]
    content: Content,
}

#[derive(Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlainVersion {
    #[schema(minimum = 1)]
    version: u32,
    version_id: VersionId,
    /// Null for the first version.
    #[schema(required = true)]
    parent_id: Option<VersionId>,
    /// RFC 3339, UTC; null for the first version backfilled by the migration.
    #[schema(required = true, format = DateTime)]
    created_at: Option<String>,
    #[schema(inline)]
    encrypted: crate::v1::model::False,
    title: Title,
}

#[derive(Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct EncryptedVersion {
    #[schema(minimum = 1)]
    version: u32,
    version_id: VersionId,
    /// Null for the first version.
    #[schema(required = true)]
    parent_id: Option<VersionId>,
    /// RFC 3339, UTC; null for the first version backfilled by the migration.
    #[schema(required = true, format = DateTime)]
    created_at: Option<String>,
    #[schema(inline)]
    encrypted: True,
}

/// One stored version, one of two closed shapes told apart by `encrypted`. An encrypted paste's versions have no `title` field.
#[derive(Serialize, ToSchema)]
#[serde(untagged)]
pub(crate) enum Version {
    Plain(PlainVersion),
    Encrypted(EncryptedVersion),
}

#[derive(Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct VersionListDocument {
    #[schema(max_items = 20)]
    versions: Vec<Version>,
}

#[derive(Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorkspaceCreate {
    slug: Slug,
    #[schema(nullable = false)]
    title: Option<WorkspaceTitle>,
}

#[derive(Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorkspaceCreateRequest {
    workspace: WorkspaceCreate,
}

#[derive(Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
struct WorkspaceUpdate {
    title: WorkspaceTitle,
}

#[derive(Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorkspaceUpdateRequest {
    #[schema(inline)]
    workspace: WorkspaceUpdate,
}

#[derive(Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorkspaceEntryAdd {
    name: EntryName,
    #[schema(nullable = false)]
    kind: Option<Kind>,
    paste_id: PasteId,
}

#[derive(Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorkspaceEntryAddRequest {
    entry: WorkspaceEntryAdd,
}

#[derive(Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlainWorkspaceEntry {
    name: EntryName,
    kind: Kind,
    paste_id: PasteId,
    #[schema(inline)]
    encrypted: crate::v1::model::False,
    title: Title,
    #[schema(minimum = 1)]
    version: u32,
    /// Time of the paste's current version, RFC 3339, UTC; null when unknown (backfilled first version).
    #[schema(required = true, format = DateTime)]
    updated_at: Option<String>,
    links: PasteLinks,
}

#[derive(Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct EncryptedWorkspaceEntry {
    name: EntryName,
    kind: Kind,
    paste_id: PasteId,
    #[schema(inline)]
    encrypted: True,
    #[schema(minimum = 1)]
    version: u32,
    /// Time of the paste's current version, RFC 3339, UTC; null when unknown (backfilled first version).
    #[schema(required = true, format = DateTime)]
    updated_at: Option<String>,
    links: PasteLinks,
}

/// One entry, one of two closed shapes told apart by `encrypted`. An entry whose paste is encrypted has no `title` field.
#[derive(Serialize, ToSchema)]
#[serde(untagged)]
pub(crate) enum WorkspaceEntry {
    Plain(PlainWorkspaceEntry),
    Encrypted(EncryptedWorkspaceEntry),
}

#[derive(Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorkspaceEntryDocument {
    entry: WorkspaceEntry,
}

#[derive(Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct Workspace {
    slug: Slug,
    title: WorkspaceTitle,
    created_at: Timestamp,
    updated_at: Timestamp,
    /// At most 500 entries; order as in `showWorkspace`.
    #[schema(max_items = 500)]
    entries: Vec<WorkspaceEntry>,
    /// `self` is the API URL; `bookmark` is `/w/<slug>`.
    links: Vec<Link>,
}

#[derive(Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorkspaceDocument {
    workspace: Workspace,
}
