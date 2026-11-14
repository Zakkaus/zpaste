//! The bodies of `readContent` and `getContent`.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use wastebin_core::db::Head;

use super::{True, VersionId};

/// `password` for an encrypted paste, `confirm_burn: true` for a
/// burn-after-reading paste, both in one request to consume an encrypted burn
/// paste. See `readContent` for every combination.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ContentReadRequest {
    #[schema(nullable = false, min_length = 1)]
    pub password: Option<String>,
    #[schema(inline, nullable = false)]
    pub confirm_burn: Option<True>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
struct Content {
    text: String,
    version_id: VersionId,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ContentDocument {
    #[schema(inline)]
    content: Content,
}

impl ContentDocument {
    pub(crate) fn new(head: &Head, text: String) -> Self {
        Self {
            content: Content {
                text,
                version_id: VersionId(head.version_id.to_string()),
            },
        }
    }
}
