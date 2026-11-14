//! Content Security Policies: upstream's strict base, which every response gets unless it brings
//! its own, and the policies derived from it (docs/architecture.md, invariant 17).

use std::sync::LazyLock;

use axum::http::HeaderValue;

/// Upstream's strict policy, directive by directive.
const BASE: [(&str, &str); 9] = [
    ("default-src", "'none'"),
    ("script-src", "'self'"),
    ("img-src", "'self' data:"),
    ("style-src", "'self' data:"),
    ("font-src", "'self' data:"),
    ("object-src", "'none'"),
    ("base-uri", "'none'"),
    ("frame-ancestors", "'none'"),
    ("form-action", "'self'"),
];

/// The policy of every response that does not bring its own.
pub(crate) static STRICT: LazyLock<HeaderValue> = LazyLock::new(|| widened(&[]));

/// The base policy with each `(directive, sources)` of `extra` added: the sources join the
/// directive's own, or the directive is appended when the base lacks it. Empty sources add nothing.
pub(crate) fn widened(extra: &[(&str, &str)]) -> HeaderValue {
    let mut directives: Vec<(&str, String)> = BASE
        .iter()
        .map(|(name, sources)| (*name, (*sources).to_owned()))
        .collect();
    for (name, sources) in extra.iter().filter(|(_, sources)| !sources.is_empty()) {
        match directives.iter_mut().find(|(known, _)| known == name) {
            Some((_, known)) => {
                known.push(' ');
                known.push_str(sources);
            }
            None => directives.push((name, (*sources).to_owned())),
        }
    }
    let policy = directives
        .iter()
        .map(|(name, sources)| format!("{name} {sources}"))
        .collect::<Vec<_>>()
        .join("; ");
    HeaderValue::try_from(policy).expect("a policy is ASCII")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widened_extends_known_directives_and_appends_new_ones() {
        let policy = widened(&[
            ("img-src", "https:"),
            ("style-src", ""),
            ("connect-src", "'self'"),
        ]);
        let policy = policy.to_str().unwrap();
        assert!(
            policy.contains("; img-src 'self' data: https:;"),
            "{policy}"
        );
        assert!(policy.contains("; style-src 'self' data:;"), "{policy}");
        assert!(
            policy.ends_with("; form-action 'self'; connect-src 'self'"),
            "{policy}"
        );
        assert_eq!(
            STRICT.to_str().unwrap(),
            "default-src 'none'; script-src 'self'; img-src 'self' data:; style-src 'self' data:; \
             font-src 'self' data:; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; \
             form-action 'self'"
        );
    }
}
