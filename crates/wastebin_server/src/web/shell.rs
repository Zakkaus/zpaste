//! The reading page's HTML shell: the web app's `index.html` with the paste's link preview, the
//! paste itself as embedded JSON and a `<noscript>` copy of its text (docs/architecture.md,
//! "Serving a page"). The JSON shape and its escaping are defined in `web/src/shell/embedded.ts`.

use std::fmt::Write as _;

use serde::Serialize;
use wastebin_core::db::Head;

/// `EMBEDDED_PASTE_ID` in `web/src/shell/embedded.ts`.
const EMBEDDED_PASTE_ID: &str = "zpaste-paste";
/// The link preview's title, in characters.
const TITLE_CHARS: usize = 80;
/// The link preview's summary: at most this many lines of the paste, and of characters.
const SUMMARY_LINES: usize = 4;
const SUMMARY_CHARS: usize = 200;

/// The web app's page, split where the shell inserts the paste. `build.rs` checks that each marker
/// occurs once.
pub(crate) struct Template {
    /// Up to `<title>`.
    head: &'static str,
    /// After `</title>`, up to the app's root element.
    middle: &'static str,
    /// From the root element on.
    body: &'static str,
}

impl Template {
    pub(crate) fn parse(index: &'static str) -> Self {
        let (head, rest) = index.split_once("<title>").expect("<title> in the page");
        let (_, rest) = rest.split_once("</title>").expect("</title> in the page");
        let root = rest
            .find("<div id=\"root\"></div>")
            .expect("the root element in the page");
        let (middle, body) = rest.split_at(root);
        Self { head, middle, body }
    }
}

/// What a reading page carries.
pub(crate) enum Reading<'a> {
    /// No paste: the id is missing, expired or not an id. The app shows its error state.
    Missing,
    /// An ordinary paste, embedded whole.
    Plain(Embedded<'a>),
}

/// `EmbeddedPaste` in `web/src/shell/embedded.ts`.
#[derive(Serialize)]
pub(crate) struct Embedded<'a> {
    pub paste: PasteMeta<'a>,
    pub content: &'a str,
    pub etag: String,
    pub url_extension: Option<&'a str>,
    pub can_edit: bool,
}

/// `PasteMeta` in `web/src/shell/embedded.ts`, the fields of `GET /v1/pastes/{id}`.
#[derive(Serialize)]
pub(crate) struct PasteMeta<'a> {
    pub id: String,
    pub extension: Option<&'a str>,
    pub title: Option<&'a str>,
    pub encrypted: bool,
    pub burn_after_reading: bool,
    pub created_at: Option<&'a str>,
    pub expires_at: Option<&'a str>,
    pub version: u32,
    pub version_id: String,
}

impl<'a> PasteMeta<'a> {
    pub(crate) fn new(head: &'a Head) -> Self {
        Self {
            id: head.id.to_string(),
            extension: head.extension.as_deref(),
            title: head.title.as_deref(),
            encrypted: head.encrypted,
            burn_after_reading: head.burn_after_reading,
            created_at: head.created_at.as_deref(),
            expires_at: head.expires_at.as_deref(),
            version: head.version,
            version_id: head.version_id.to_string(),
        }
    }
}

/// The page for `reading` at `url`. `site` names the service in titles and previews;
/// `noscript_css` is the route of the `<noscript>` stylesheet.
pub(crate) fn render(
    template: &Template,
    reading: &Reading<'_>,
    site: &str,
    url: &str,
    noscript_css: &str,
) -> String {
    let mut head = String::new();
    let mut body = String::new();
    let site = escape(site);

    match reading {
        Reading::Missing => {
            let _ = write!(head, "<title>{site}</title>");
            let _ = write!(
                head,
                "<meta property=\"og:site_name\" content=\"{site}\" />\
                 <meta property=\"og:title\" content=\"{site}\" />\
                 <meta name=\"twitter:card\" content=\"summary\" />"
            );
        }
        Reading::Plain(embedded) => {
            let (title, summary) = preview(embedded.paste.title, embedded.content);
            let (title, summary) = (escape(&title), escape(&summary));
            let _ = write!(head, "<title>{title} · {site}</title>");
            let _ = write!(
                head,
                "<meta property=\"og:site_name\" content=\"{site}\" />\
                 <meta property=\"og:type\" content=\"article\" />\
                 <meta property=\"og:url\" content=\"{}\" />\
                 <meta property=\"og:title\" content=\"{title}\" />\
                 <meta name=\"twitter:card\" content=\"summary\" />\
                 <meta name=\"twitter:title\" content=\"{title}\" />",
                escape(url)
            );
            if !summary.is_empty() {
                let _ = write!(
                    head,
                    "<meta name=\"description\" content=\"{summary}\" />\
                     <meta property=\"og:description\" content=\"{summary}\" />\
                     <meta name=\"twitter:description\" content=\"{summary}\" />"
                );
            }
            let _ = write!(
                head,
                "<noscript><link rel=\"stylesheet\" href=\"{noscript_css}\" /></noscript>"
            );
            let _ = write!(
                body,
                "<script type=\"application/json\" id=\"{EMBEDDED_PASTE_ID}\">{}</script>\
                 <noscript><pre>{}</pre></noscript>",
                script_json(embedded),
                escape(embedded.content)
            );
        }
    }

    [template.head, &head, template.middle, &body, template.body].concat()
}

/// JSON safe between `<script type="application/json">` and `</script>`: `<`, `>`, `&`, U+2028 and
/// U+2029 become `\u` escapes (invariant 5), so no paste can end or alter the element.
/// `serializeEmbeddedPaste` in `web/src/shell/embedded.ts` writes the same.
pub(crate) fn script_json(value: &impl Serialize) -> String {
    let json = serde_json::to_string(value).expect("the paste serializes");
    let mut safe = String::with_capacity(json.len());
    for c in json.chars() {
        match c {
            '<' | '>' | '&' | '\u{2028}' | '\u{2029}' => {
                let _ = write!(safe, "\\u{:04x}", u32::from(c));
            }
            c => safe.push(c),
        }
    }
    safe
}

/// Text safe in HTML content and in a quoted attribute value.
pub(crate) fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            c => escaped.push(c),
        }
    }
    escaped
}

/// The link preview of a paste: its title, else its first line (without a Markdown heading's
/// `#`), and a summary of the lines after that.
fn preview(title: Option<&str>, content: &str) -> (String, String) {
    let mut lines = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty());
    let title = match title.filter(|title| !title.trim().is_empty()) {
        Some(title) => title.trim().to_owned(),
        None => lines
            .next()
            .map(|line| {
                let heading = line.trim_start_matches('#');
                if heading.len() < line.len()
                    && heading.starts_with(' ')
                    && line.len() - heading.len() <= 6
                {
                    heading.trim_start()
                } else {
                    line
                }
            })
            .unwrap_or_default()
            .to_owned(),
    };
    let summary = lines.take(SUMMARY_LINES).collect::<Vec<_>>().join("\n");
    (
        shorten(&title, TITLE_CHARS),
        shorten(&summary, SUMMARY_CHARS),
    )
}

/// `text` cut to at most `max` characters, the last of them an ellipsis when cut.
fn shorten(text: &str, max: usize) -> String {
    match text.char_indices().nth(max) {
        None => text.to_owned(),
        Some(_) => {
            let mut short: String = text.chars().take(max - 1).collect();
            short.push('…');
            short
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_json_cannot_end_the_element() {
        let content = "</script><script>alert(1)</script><!-- & \u{2028}\u{2029} \"x\"";
        let json = script_json(&serde_json::json!({ "content": content }));

        assert!(
            !json.contains('<') && !json.contains('>') && !json.contains('&'),
            "{json}"
        );
        assert!(
            !json.contains('\u{2028}') && !json.contains('\u{2029}'),
            "{json}"
        );
        assert!(json.contains(r"\u003c/script\u003e"), "{json}");
        let back: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(back["content"], content);
    }

    #[test]
    fn escape_covers_text_and_attributes() {
        assert_eq!(
            escape(r#"<a href="x">'&'</a>"#),
            "&lt;a href=&quot;x&quot;&gt;&#39;&amp;&#39;&lt;/a&gt;"
        );
    }

    #[test]
    fn preview_titles_and_summaries() {
        let markdown = "# Plan\n\nFirst line.\n  Second line.\n\nThird.\nFourth.\nFifth.\n";
        assert_eq!(
            preview(None, markdown),
            (
                String::from("Plan"),
                String::from("First line.\nSecond line.\nThird.\nFourth.")
            )
        );
        assert_eq!(
            preview(Some("Notes"), "one\ntwo"),
            (String::from("Notes"), String::from("one\ntwo"))
        );
        // Not a heading: a shebang and a hashtag keep their `#`.
        assert_eq!(preview(None, "#!/bin/sh\nexit 0").0, "#!/bin/sh");
        assert_eq!(preview(None, "#tag").0, "#tag");
        assert_eq!(preview(None, "").0, "");

        let long = "x".repeat(500);
        let (title, summary) = preview(Some(&long), &long);
        assert_eq!(title.chars().count(), TITLE_CHARS);
        assert!(title.ends_with('…'));
        assert_eq!(summary.chars().count(), SUMMARY_CHARS);
    }

    #[test]
    fn template_splits_at_the_markers() {
        let template = Template::parse(super::super::dist::INDEX);
        assert!(template.head.contains("<head>"));
        assert!(template.body.starts_with("<div id=\"root\"></div>"));
        assert!(!template.middle.contains("<title>"));
    }
}
