//! Validation against the generated contract with the `jsonschema` crate: JSON Schema draft
//! 2020-12, the dialect of OpenAPI 3.1, with `format` asserted and unknown formats refused. Every
//! schema is checked against the draft's meta-schema when it is compiled, so a malformed keyword
//! fails even in a `oneOf` branch or a property the instance leaves out.

use serde_json::{Value, json};

/// Validate `instance` against `schema`, resolving `$ref`s in `document`.
pub(crate) fn validate(document: &Value, schema: &Value, instance: &Value) -> Result<(), String> {
    // The document is the root resource, so `#/components/...` resolves in it; the schema under
    // test sits at `/x-subject`. The OpenAPI fields at the root are not JSON Schema keywords, so
    // they assert nothing.
    let mut root = document.clone();
    root["x-subject"] = schema.clone();
    root["$ref"] = json!("#/x-subject");

    let validator = jsonschema::draft202012::options()
        .should_validate_formats(true)
        .should_ignore_unknown_formats(false)
        // OpenAPI's integer formats; JSON Schema checks formats on strings only, so these
        // never see a value.
        .with_format("int32", |_| false)
        .with_format("int64", |_| false)
        .build(&root)
        .map_err(|err| format!("invalid schema: {err}"))?;

    let errors: Vec<String> = validator
        .iter_errors(instance)
        .map(|err| format!("{}: {err}", err.instance_path()))
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

/// The value at a local `$ref` such as `#/components/schemas/Paste`.
pub(crate) fn resolve<'d>(document: &'d Value, reference: &str) -> Result<&'d Value, String> {
    reference
        .strip_prefix('#')
        .and_then(|pointer| document.pointer(pointer))
        .ok_or_else(|| format!("unresolved $ref {reference}"))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::validate;
    use crate::v1::contract::document;

    fn paste_document(paste: serde_json::Value) -> Result<(), String> {
        let document = document();
        let schema = json!({"$ref": "#/components/schemas/PasteDocument"});
        validate(&document, &schema, &json!({"paste": paste}))
    }

    fn plain() -> serde_json::Value {
        json!({
            "id": "abc123",
            "extension": null,
            "title": "t",
            "encrypted": false,
            "burn_after_reading": false,
            "expires_at": null,
            "version": 1,
            "version_id": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
            "links": [{"rel": "self", "href": "https://example.org/v1/pastes/abc123"}],
        })
    }

    #[test]
    fn accepts_a_valid_paste() {
        assert_eq!(paste_document(plain()), Ok(()));
    }

    /// Each planted violation must fail validation.
    #[test]
    fn rejects_planted_violations() {
        let mut leaked = plain();
        leaked["uid"] = json!(1);
        let mut encrypted_title = plain();
        encrypted_title["encrypted"] = json!(true);
        let mut missing = plain();
        missing
            .as_object_mut()
            .map(|object| object.remove("expires_at"));
        let mut bad_id = plain();
        bad_id["id"] = json!("abc");
        let mut bad_version = plain();
        bad_version["version"] = json!(0);
        let mut long_title = plain();
        long_title["title"] = json!("x".repeat(257));
        let mut bad_rel = plain();
        bad_rel["links"][0]["rel"] = json!("edit");

        for (name, paste) in [
            ("leaked field", leaked),
            ("encrypted with title", encrypted_title),
            ("missing required", missing),
            ("pattern", bad_id),
            ("minimum", bad_version),
            ("maxLength", long_title),
            ("enum", bad_rel),
        ] {
            assert!(paste_document(paste).is_err(), "{name} passed");
        }
    }

    /// A violation nested in a `oneOf` branch, in an optional property, or of a `format` fails.
    #[test]
    fn rejects_nested_and_format_violations() {
        let document = document();
        let created = json!({"$ref": "#/components/schemas/PasteCreatedDocument"});

        let mut encrypted = plain();
        encrypted["encrypted"] = json!(true);
        encrypted
            .as_object_mut()
            .map(|object| object.remove("title"));
        assert_eq!(paste_document(encrypted.clone()), Ok(()));

        let mut bad_branch = encrypted;
        bad_branch["extension"] = json!(5);
        let mut bad_token = plain();
        bad_token["owner_token"] = json!("token");
        assert_eq!(
            validate(&document, &created, &json!({"paste": bad_token.clone()})),
            Ok(())
        );
        bad_token["owner_token"] = json!(5);
        let mut bad_date = plain();
        bad_date["expires_at"] = json!("tomorrow");
        let mut bad_uri = plain();
        bad_uri["links"][0]["href"] = json!("no scheme");

        for (name, paste) in [
            ("oneOf branch", bad_branch),
            ("date-time", bad_date),
            ("uri", bad_uri),
        ] {
            assert!(paste_document(paste).is_err(), "{name} passed");
        }
        assert!(
            validate(&document, &created, &json!({"paste": bad_token})).is_err(),
            "optional property passed"
        );
    }

    /// A malformed schema fails even where the instance never reaches it.
    #[test]
    fn rejects_malformed_schemas_anywhere() {
        for schema in [
            json!({"oneOf": [{"type": "string"}, {"type": "string", "maxLength": "ten"}]}),
            json!({"type": "object", "properties": {"absent": {"type": "strin"}}}),
            json!({"type": "string", "format": "date-tim"}),
        ] {
            assert!(
                validate(&json!({}), &schema, &json!({})).is_err()
                    && validate(&json!({}), &schema, &json!("x")).is_err(),
                "{schema} passed"
            );
        }
    }
}
