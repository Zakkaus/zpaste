//! Embeds the built web app (`web/dist`, from `pnpm build`) in the executable.
//!
//! Writes `$OUT_DIR/web_dist.rs` with the page template, the files to serve, the style hashes of
//! the reading page's Content Security Policy (`style-src.txt`) and those of each SVG file's own
//! inline styles (`svg-style-src.txt`), both from `web/scripts/style-hashes.ts`.
//! `ZPASTE_WEB_DIST` names another build directory. Without a build the executable serves a
//! placeholder page and says so at build time, so `cargo build` and `cargo test` work without Node;
//! `ZPASTE_REQUIRE_WEB=1` (set by the Dockerfile) turns that into an error.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::{env, fs};

/// Strings the server splits the page template at (`src/web/shell.rs`); each occurs exactly once.
const MARKERS: [&str; 3] = ["<title>", "</title>", "<div id=\"root\"></div>"];
/// The hashes files, which the server reads here and does not serve.
const STYLE_SRC: &str = "style-src.txt";
const SVG_STYLE_SRC: &str = "svg-style-src.txt";

fn main() -> Result<(), String> {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let dist = env::var_os("ZPASTE_WEB_DIST")
        .map_or_else(|| manifest.join("../../web/dist"), PathBuf::from);
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("web_dist.rs");

    println!("cargo::rerun-if-env-changed=ZPASTE_WEB_DIST");
    println!("cargo::rerun-if-env-changed=ZPASTE_REQUIRE_WEB");
    println!("cargo::rerun-if-changed={}", dist.display());

    let index = dist.join("index.html");
    let code = if index.is_file() {
        built(&dist, &index)?
    } else {
        if env::var("ZPASTE_REQUIRE_WEB").is_ok_and(|value| value == "1") {
            return Err(format!(
                "{} not found; run `pnpm build` in web/",
                index.display()
            ));
        }
        println!(
            "cargo::warning={} not found: the reading page is a placeholder; run `pnpm build` in web/ first",
            index.display()
        );
        let placeholder = manifest.join("src/web/placeholder.html");
        check_markers(&placeholder);
        format!(
            "pub(crate) const BUILT: bool = false;\n\
             pub(crate) const INDEX: &str = include_str!({placeholder:?});\n\
             pub(crate) const STYLE_SRC: &[&str] = &[];\n\
             pub(crate) const SVG_STYLE_SRC: &[(&str, &[&str])] = &[];\n\
             pub(crate) const FILES: &[(&str, &[u8])] = &[];\n"
        )
    };

    fs::write(out, code).map_err(|err| format!("write web_dist.rs: {err}"))
}

/// The generated code for the build in `dist`.
fn built(dist: &Path, index: &Path) -> Result<String, String> {
    check_markers(index);

    let style_src = read_hashes(dist, STYLE_SRC)?;
    let sources: Vec<&str> = style_src.lines().filter(|line| !line.is_empty()).collect();
    check_sources(STYLE_SRC, &sources);
    let svg_style_src = read_hashes(dist, SVG_STYLE_SRC)?;
    let svg_sources: Vec<(&str, Vec<&str>)> = svg_style_src
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            Some((fields.next()?, fields.collect()))
        })
        .collect();
    for (_, sources) in &svg_sources {
        check_sources(SVG_STYLE_SRC, sources);
    }

    let mut files = Vec::new();
    collect(dist, dist, &mut files);
    files.sort();

    let mut code = String::from("pub(crate) const BUILT: bool = true;\n");
    let _ = writeln!(
        code,
        "pub(crate) const INDEX: &str = include_str!({index:?});"
    );
    let _ = writeln!(code, "pub(crate) const STYLE_SRC: &[&str] = &{sources:?};");
    code.push_str("pub(crate) const SVG_STYLE_SRC: &[(&str, &[&str])] = &[\n");
    for (route, sources) in svg_sources {
        let _ = writeln!(code, "    ({route:?}, &{sources:?}),");
    }
    code.push_str("];\n");
    code.push_str("pub(crate) const FILES: &[(&str, &[u8])] = &[\n");
    for (route, path) in files {
        let _ = writeln!(code, "    ({route:?}, include_bytes!({path:?})),");
    }
    code.push_str("];\n");
    Ok(code)
}

/// Every file under `dir` except the template and the hashes, as `(route, path)`.
fn collect(root: &Path, dir: &Path, files: &mut Vec<(String, PathBuf)>) {
    for entry in fs::read_dir(dir).expect("read web/dist") {
        let path = entry.expect("read web/dist").path();
        if path.is_dir() {
            collect(root, &path, files);
            continue;
        }
        let relative = path.strip_prefix(root).expect("under web/dist");
        let route = format!("/{}", relative.to_str().expect("UTF-8 file name"));
        let hashes = [STYLE_SRC, SVG_STYLE_SRC].map(|name| format!("/{name}"));
        if route != "/index.html" && !hashes.contains(&route) {
            files.push((route, path));
        }
    }
}

fn read_hashes(dist: &Path, name: &str) -> Result<String, String> {
    fs::read_to_string(dist.join(name)).map_err(|err| {
        format!(
            "{}: {err}; build web/ with `pnpm build`",
            dist.join(name).display()
        )
    })
}

fn check_sources(file: &str, sources: &[&str]) {
    for source in sources {
        let valid = source
            .strip_prefix("'sha256-")
            .and_then(|rest| rest.strip_suffix('\''))
            .is_some_and(|hash| {
                hash.len() == 44
                    && hash
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'='))
            });
        assert!(valid, "{file}: not a sha256 source: {source}");
    }
}

fn check_markers(index: &Path) {
    let html = fs::read_to_string(index).expect("read the page template");
    for marker in MARKERS {
        assert!(
            html.matches(marker).count() == 1,
            "{}: `{marker}` must occur exactly once; the shell inserts the paste there",
            index.display()
        );
    }
}
