//! A JSON Schema validator for the subset of keywords the contract uses. A keyword outside the
//! subset is an error, so a schema the validator does not understand never passes silently.

use serde_json::Value;

/// Keywords that only annotate.
const ANNOTATIONS: &[&str] = &[
    "description",
    "example",
    "examples",
    "default",
    "format",
    "title",
];

/// Validate `instance` against `schema`, resolving `$ref`s in `document`.
pub(crate) fn validate(document: &Value, schema: &Value, instance: &Value) -> Result<(), String> {
    check(document, schema, instance, "")
}

/// The value at a local `$ref` such as `#/components/schemas/Paste`.
pub(crate) fn resolve<'d>(document: &'d Value, reference: &str) -> Result<&'d Value, String> {
    reference
        .strip_prefix('#')
        .and_then(|pointer| document.pointer(pointer))
        .ok_or_else(|| format!("unresolved $ref {reference}"))
}

fn check(document: &Value, schema: &Value, instance: &Value, at: &str) -> Result<(), String> {
    let schema = schema
        .as_object()
        .ok_or_else(|| format!("{at}: schema is not an object"))?;

    for (keyword, value) in schema {
        match keyword.as_str() {
            "$ref" => {
                let target = resolve(document, value.as_str().unwrap_or_default())?;
                check(document, target, instance, at)?;
            }
            "type" => {
                let types: Vec<&str> = match value {
                    Value::String(name) => vec![name.as_str()],
                    Value::Array(names) => names.iter().filter_map(Value::as_str).collect(),
                    _ => return Err(format!("{at}: malformed type")),
                };
                if !types.iter().any(|name| has_type(instance, name)) {
                    return Err(format!("{at}: {instance} is not of type {types:?}"));
                }
            }
            "enum" => {
                if !value
                    .as_array()
                    .is_some_and(|values| values.contains(instance))
                {
                    return Err(format!("{at}: {instance} is not one of {value}"));
                }
            }
            "oneOf" => {
                let matches = value
                    .as_array()
                    .ok_or_else(|| format!("{at}: malformed oneOf"))?
                    .iter()
                    .filter(|option| check(document, option, instance, at).is_ok())
                    .count();
                if matches != 1 {
                    return Err(format!("{at}: {matches} oneOf branches match {instance}"));
                }
            }
            "required" => {
                if let Some(object) = instance.as_object() {
                    for name in value.as_array().into_iter().flatten() {
                        let name = name.as_str().unwrap_or_default();
                        if !object.contains_key(name) {
                            return Err(format!("{at}: missing required {name}"));
                        }
                    }
                }
            }
            "properties" => {
                if let Some(object) = instance.as_object() {
                    for (name, property) in value.as_object().into_iter().flatten() {
                        if let Some(field) = object.get(name) {
                            check(document, property, field, &format!("{at}/{name}"))?;
                        }
                    }
                }
            }
            "additionalProperties" => {
                if value != &Value::Bool(false) {
                    return Err(format!(
                        "{at}: only additionalProperties: false is supported"
                    ));
                }
                if let Some(object) = instance.as_object() {
                    let known = schema.get("properties").and_then(Value::as_object);
                    if let Some(name) = object
                        .keys()
                        .find(|name| !known.is_some_and(|known| known.contains_key(*name)))
                    {
                        return Err(format!("{at}: field {name} is not in the schema"));
                    }
                }
            }
            "items" => {
                for (index, item) in instance.as_array().into_iter().flatten().enumerate() {
                    check(document, value, item, &format!("{at}/{index}"))?;
                }
            }
            "minItems" | "maxItems" => {
                if let Some(items) = instance.as_array() {
                    bound(keyword, value, items.len(), at)?;
                }
            }
            "minLength" | "maxLength" => {
                if let Some(text) = instance.as_str() {
                    bound(keyword, value, text.chars().count(), at)?;
                }
            }
            "minimum" | "maximum" => {
                if let (Some(limit), Some(number)) = (value.as_f64(), instance.as_f64()) {
                    let fits = if keyword == "minimum" {
                        number >= limit
                    } else {
                        number <= limit
                    };
                    if !fits {
                        return Err(format!("{at}: {number} violates {keyword} {limit}"));
                    }
                }
            }
            "pattern" => {
                if let Some(text) = instance.as_str() {
                    let pattern = value.as_str().unwrap_or_default();
                    let regex = fancy_regex::Regex::new(pattern)
                        .map_err(|err| format!("{at}: bad pattern {pattern}: {err}"))?;
                    if !regex.is_match(text).unwrap_or(false) {
                        return Err(format!("{at}: {text:?} does not match {pattern}"));
                    }
                }
            }
            keyword if ANNOTATIONS.contains(&keyword) => {}
            keyword => return Err(format!("{at}: unsupported keyword {keyword}")),
        }
    }

    Ok(())
}

fn has_type(instance: &Value, name: &str) -> bool {
    match name {
        "null" => instance.is_null(),
        "boolean" => instance.is_boolean(),
        "string" => instance.is_string(),
        "integer" => instance.is_i64() || instance.is_u64(),
        "number" => instance.is_number(),
        "array" => instance.is_array(),
        "object" => instance.is_object(),
        _ => false,
    }
}

fn bound(keyword: &str, limit: &Value, actual: usize, at: &str) -> Result<(), String> {
    let limit = limit
        .as_u64()
        .and_then(|limit| usize::try_from(limit).ok())
        .ok_or_else(|| format!("{at}: malformed {keyword}"))?;
    let fits = if keyword.starts_with("min") {
        actual >= limit
    } else {
        actual <= limit
    };

    if fits {
        Ok(())
    } else {
        Err(format!("{at}: {actual} violates {keyword} {limit}"))
    }
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

    #[test]
    fn rejects_unknown_keywords() {
        let schema = json!({"type": "string", "contentEncoding": "base64"});
        assert!(validate(&json!({}), &schema, &json!("x")).is_err());
    }
}
