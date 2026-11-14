//! The body of `listPastes`.

use serde::Serialize;
use url::Url;
use utoipa::ToSchema;

use wastebin_core::db::Head;

use super::{Link, LinkRel, Paste};

#[derive(Debug, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct PasteListDocument {
    pastes: Vec<Paste>,
    /// Holds one `next` link while more pages exist; empty on the last page.
    #[schema(max_items = 1, example = json!([
        {"rel": "next", "href": "https://paste.gentoozh.org/v1/pastes?owner=me&limit=20&marker=abc123"}
    ]))]
    pastes_links: Vec<Link>,
}

impl PasteListDocument {
    /// `heads` as one page; `next` is the query of the following page, when there is one.
    pub(crate) fn new(heads: &[Head], base_url: &Url, next: Option<String>) -> Self {
        let base = base_url.as_str().trim_end_matches('/');

        Self {
            pastes: heads
                .iter()
                .map(|head| Paste::new(head, base_url))
                .collect(),
            pastes_links: next
                .map(|query| Link {
                    rel: LinkRel::Next,
                    href: format!("{base}/v1/pastes?{query}"),
                })
                .into_iter()
                .collect(),
        }
    }
}
