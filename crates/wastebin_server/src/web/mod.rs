//! The web app built from `web/` and embedded by `build.rs` (docs/architecture.md, "Serving a
//! page"): its files, served from memory, and the reading page's shell and policy.

pub(crate) mod shell;

use std::sync::LazyLock;

use axum::body::Bytes;
use axum::http::HeaderValue;

use crate::assets::{Asset, Kind};

mod dist {
    include!(concat!(env!("OUT_DIR"), "/web_dist.rs"));
}

/// Vite names the files under this directory after their content.
const HASHED_DIR: &str = "/assets/";

/// The reading page's Content Security Policy without its style hashes: upstream's strict base
/// plus `connect-src 'self'` and the images of invariant 17.
const READING_CSP: [&str; 2] = [
    "default-src 'none'; script-src 'self'; img-src 'self' https: data:; style-src 'self' data:",
    "; font-src 'self' data:; connect-src 'self'; object-src 'none'; base-uri 'none'; \
     frame-ancestors 'none'; form-action 'self'",
];

/// Wraps the plain text of a `<noscript>` copy instead of scrolling sideways.
const NOSCRIPT_CSS: &str = "pre{white-space:pre-wrap;overflow-wrap:anywhere;margin:1rem}";

pub(crate) struct Web {
    /// Every file of the build but the page template.
    pub files: Vec<Asset>,
    /// The `<noscript>` stylesheet of a reading page.
    pub noscript_css: Asset,
    pub template: shell::Template,
    /// The reading page's Content Security Policy.
    pub csp: HeaderValue,
}

/// The embedded build, prepared (and compressed) once per process.
pub(crate) static WEB: LazyLock<Web> = LazyLock::new(|| {
    let files = dist::FILES
        .iter()
        .map(|(route, content)| {
            let (mime, compressible) = mime_of(route);
            Asset::at(
                (*route).to_owned(),
                mime,
                Bytes::from_static(content),
                compressible,
                route.starts_with(HASHED_DIR),
            )
        })
        .collect();
    let csp = format!(
        "{}{}{}",
        READING_CSP[0],
        dist::STYLE_SRC
            .iter()
            .fold(String::new(), |sources, source| sources + " " + source),
        READING_CSP[1]
    );

    Web {
        files,
        noscript_css: Asset::new("noscript", Kind::Css, NOSCRIPT_CSS.as_bytes().to_vec()),
        template: shell::Template::parse(dist::INDEX),
        csp: HeaderValue::try_from(csp).expect("the policy is ASCII"),
    }
});

/// Whether the executable embeds a build of the web app rather than the placeholder page.
pub(crate) const BUILT: bool = dist::BUILT;

/// The type of a file of the build, and whether a content coding shrinks it.
fn mime_of(route: &str) -> (mime::Mime, bool) {
    let extension = route
        .rsplit_once('.')
        .map_or("", |(_, extension)| extension);
    let (mime, compressible) = match extension {
        "js" | "mjs" => ("text/javascript", true),
        "css" => ("text/css", true),
        "html" => ("text/html; charset=utf-8", true),
        "json" => ("application/json", true),
        "webmanifest" => ("application/manifest+json", true),
        "svg" => ("image/svg+xml", true),
        "txt" => ("text/plain; charset=utf-8", true),
        "ico" => ("image/x-icon", true),
        "png" => ("image/png", false),
        "woff2" => ("font/woff2", false),
        "woff" => ("font/woff", false),
        _ => ("application/octet-stream", false),
    };
    (mime.parse().expect("a valid media type"), compressible)
}

#[cfg(test)]
mod tests;
