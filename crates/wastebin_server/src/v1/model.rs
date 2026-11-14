//! Request and response bodies of the `/v1` operations built so far. Each response shape of the
//! contract is its own closed struct, never the storage struct, so a field the contract does not
//! list cannot leak.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use url::Url;
use utoipa::openapi::RefOr;
use utoipa::openapi::schema::{ObjectBuilder, Schema, Type};
use utoipa::{PartialSchema, ToSchema};

use wastebin_core::db::Head;

mod content;
mod list;

pub(crate) use content::{ContentDocument, ContentReadRequest};
pub(crate) use list::PasteListDocument;

/// A boolean that is always the same value, schema `enum: [<value>]` (utoipa has no `const`).
macro_rules! constant_bool {
    ($name:ident, $value:literal) => {
        #[derive(Clone, Copy, Debug, Default)]
        pub(crate) struct $name;

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_bool($value)
            }
        }

        impl PartialSchema for $name {
            fn schema() -> RefOr<Schema> {
                ObjectBuilder::new()
                    .schema_type(Type::Boolean)
                    .enum_values(Some([$value]))
                    .into()
            }
        }

        impl ToSchema for $name {}

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                if bool::deserialize(deserializer)? == $value {
                    Ok(Self)
                } else {
                    Err(D::Error::custom(concat!("expected ", stringify!($value))))
                }
            }
        }
    };
}

constant_bool!(False, false);
constant_bool!(True, true);

#[derive(Debug, Serialize, ToSchema)]
enum ApiVersionId {
    #[serde(rename = "v1")]
    V1,
}

#[derive(Debug, Serialize, ToSchema)]
enum ApiVersionStatus {
    #[serde(rename = "CURRENT")]
    Current,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
struct ApiVersion {
    #[schema(inline)]
    id: ApiVersionId,
    #[schema(inline)]
    status: ApiVersionStatus,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct VersionDocument {
    #[schema(inline)]
    version: ApiVersion,
}

pub(crate) const VERSION_DOCUMENT: VersionDocument = VersionDocument {
    version: ApiVersion {
        id: ApiVersionId::V1,
        status: ApiVersionStatus::Current,
    },
};

/// 6 or 11 characters from upstream's alphabet, without extension.
#[derive(Debug, Serialize, ToSchema)]
#[serde(transparent)]
#[schema(pattern = "^([A-Za-z0-9+-]{6}|[A-Za-z0-9+-]{11})$", example = "abc123")]
pub(crate) struct PasteId(String);

/// Hash-chain id of a version (plan section 四), 64 lowercase hex digits.
#[derive(Debug, Serialize, ToSchema)]
#[serde(transparent)]
#[schema(pattern = "^[0-9a-f]{64}$")]
pub(crate) struct VersionId(String);

/// Upstream's rule, file names such as `CMakeLists.txt` included.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(transparent)]
#[schema(pattern = "^[A-Za-z0-9._-]{1,32}$", example = "md")]
pub(crate) struct Extension(pub String);

/// At most 256 characters; null means no title.
#[derive(Debug, Default, Deserialize, Serialize, ToSchema)]
#[serde(transparent)]
#[schema(max_length = 256)]
pub(crate) struct Title(pub Option<String>);

/// RFC 3339, UTC; null when the paste never expires.
#[derive(Debug, Serialize, ToSchema)]
#[serde(transparent)]
#[schema(format = DateTime)]
pub(crate) struct ExpiresAt(Option<String>);

/// Signed uid of the new identity. Shown once; never logged.
#[derive(Serialize, ToSchema)]
#[serde(transparent)]
#[schema(min_length = 1)]
pub(crate) struct OwnerToken(String);

impl std::fmt::Debug for OwnerToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OwnerToken(..)")
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LinkRel {
    #[serde(rename = "self")]
    Self_,
    Bookmark,
    Next,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct Link {
    #[schema(inline)]
    rel: LinkRel,
    #[schema(format = "uri")]
    href: String,
}

/// `self` is the API URL; `bookmark` is the reading page, `/<id>.<ext>` when the paste has an extension.
#[derive(Debug, Serialize, ToSchema)]
#[serde(transparent)]
#[schema(example = json!([
    {"rel": "self", "href": "https://paste.gentoozh.org/v1/pastes/abc123"},
    {"rel": "bookmark", "href": "https://paste.gentoozh.org/abc123.md"}
]))]
pub(crate) struct PasteLinks(Vec<Link>);

impl PasteLinks {
    fn new(base_url: &Url, head: &Head) -> Self {
        let base = base_url.as_str().trim_end_matches('/');
        let bookmark = match &head.extension {
            Some(extension) => format!("{base}/{}.{extension}", head.id),
            None => format!("{base}/{}", head.id),
        };

        Self(vec![
            Link {
                rel: LinkRel::Self_,
                href: format!("{base}/v1/pastes/{}", head.id),
            },
            Link {
                rel: LinkRel::Bookmark,
                href: bookmark,
            },
        ])
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlainPaste {
    id: PasteId,
    /// Stored at creation; null for pastes from before the column.
    #[schema(required = true)]
    extension: Option<String>,
    title: Title,
    #[schema(inline)]
    encrypted: False,
    burn_after_reading: bool,
    expires_at: ExpiresAt,
    /// Depth in the version chain, for ordering and display only.
    #[schema(minimum = 1)]
    version: u32,
    version_id: VersionId,
    links: PasteLinks,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct EncryptedPaste {
    id: PasteId,
    /// Stored at creation; null for pastes from before the column.
    #[schema(required = true)]
    extension: Option<String>,
    #[schema(inline)]
    encrypted: True,
    burn_after_reading: bool,
    expires_at: ExpiresAt,
    /// Depth in the version chain, for ordering and display only.
    #[schema(minimum = 1)]
    version: u32,
    version_id: VersionId,
    links: PasteLinks,
}

/// Whitelisted metadata, one of two closed shapes told apart by `encrypted`.
/// Never carries the uid or an owner token; an encrypted paste has no
/// `title` field.
#[derive(Debug, Serialize, ToSchema)]
#[serde(untagged)]
pub(crate) enum Paste {
    Plain(PlainPaste),
    Encrypted(EncryptedPaste),
}

impl Paste {
    pub(crate) fn new(head: &Head, base_url: &Url) -> Self {
        let id = PasteId(head.id.to_string());
        let extension = head.extension.clone();
        let expires_at = ExpiresAt(head.expires_at.clone());
        let version_id = VersionId(head.version_id.to_string());
        let links = PasteLinks::new(base_url, head);

        if head.encrypted {
            Self::Encrypted(EncryptedPaste {
                id,
                extension,
                encrypted: True,
                burn_after_reading: head.burn_after_reading,
                expires_at,
                version: head.version,
                version_id,
                links,
            })
        } else {
            Self::Plain(PlainPaste {
                id,
                extension,
                title: Title(head.title.clone()),
                encrypted: False,
                burn_after_reading: head.burn_after_reading,
                expires_at,
                version: head.version,
                version_id,
                links,
            })
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlainPasteCreated {
    id: PasteId,
    #[schema(required = true)]
    extension: Option<String>,
    title: Title,
    #[schema(inline)]
    encrypted: False,
    burn_after_reading: bool,
    expires_at: ExpiresAt,
    #[schema(minimum = 1)]
    version: u32,
    version_id: VersionId,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    owner_token: Option<OwnerToken>,
    links: PasteLinks,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct EncryptedPasteCreated {
    id: PasteId,
    #[schema(required = true)]
    extension: Option<String>,
    #[schema(inline)]
    encrypted: True,
    burn_after_reading: bool,
    expires_at: ExpiresAt,
    #[schema(minimum = 1)]
    version: u32,
    version_id: VersionId,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    owner_token: Option<OwnerToken>,
    links: PasteLinks,
}

/// A paste as returned by create: the `Paste` fields plus `owner_token`,
/// which is present only when a non-browser create minted a new identity.
/// It is shown once and never logged.
#[derive(Debug, Serialize, ToSchema)]
#[serde(untagged)]
pub(crate) enum PasteCreated {
    Plain(PlainPasteCreated),
    Encrypted(EncryptedPasteCreated),
}

impl PasteCreated {
    pub(crate) fn new(head: &Head, base_url: &Url, owner_token: Option<String>) -> Self {
        let owner_token = owner_token.map(OwnerToken);

        match Paste::new(head, base_url) {
            Paste::Plain(paste) => Self::Plain(PlainPasteCreated {
                id: paste.id,
                extension: paste.extension,
                title: paste.title,
                encrypted: False,
                burn_after_reading: paste.burn_after_reading,
                expires_at: paste.expires_at,
                version: paste.version,
                version_id: paste.version_id,
                owner_token,
                links: paste.links,
            }),
            Paste::Encrypted(paste) => Self::Encrypted(EncryptedPasteCreated {
                id: paste.id,
                extension: paste.extension,
                encrypted: True,
                burn_after_reading: paste.burn_after_reading,
                expires_at: paste.expires_at,
                version: paste.version,
                version_id: paste.version_id,
                owner_token,
                links: paste.links,
            }),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct PasteDocument {
    pub paste: Paste,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(example = json!({"paste": {
    "id": "abc123",
    "extension": "md",
    "title": "Plan",
    "encrypted": false,
    "burn_after_reading": false,
    "expires_at": "2026-11-21T13:00:00Z",
    "version": 1,
    "version_id": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
    "owner_token": "example-owner-token",
    "links": [
        {"rel": "self", "href": "https://paste.gentoozh.org/v1/pastes/abc123"},
        {"rel": "bookmark", "href": "https://paste.gentoozh.org/abc123.md"}
    ]
}}))]
pub(crate) struct PasteCreatedDocument {
    pub paste: PasteCreated,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct PasteCreate {
    /// UTF-8; its size in bytes is at most `WASTEBIN_MAX_BODY_SIZE` (64 MiB in production).
    pub content: String,
    #[schema(nullable = false)]
    pub extension: Option<Extension>,
    #[serde(default)]
    pub title: Title,
    /// Seconds. Must be one of the values in `WASTEBIN_PASTE_EXPIRATIONS`;
    /// omitted or null takes the value marked default there (7 days in
    /// production). 0 means never, and only where that list allows it.
    #[schema(minimum = 0, example = 604800)]
    pub expires_in: Option<u64>,
    #[serde(default)]
    #[schema(default = false)]
    pub burn_after_reading: bool,
    /// When set, the content is encrypted with it.
    #[schema(min_length = 1)]
    pub password: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct PasteCreateRequest {
    pub paste: PasteCreate,
}
