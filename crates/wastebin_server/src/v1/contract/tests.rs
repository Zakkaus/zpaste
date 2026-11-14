use std::collections::BTreeSet;

use serde_json::Value;
use utoipa::OpenApi;

use super::{DOCUMENT_PATH, Pending, Routed, document, document_yaml};
use crate::v1::ROUTES;

/// Up to `limit` JSON pointers where `left` and `right` differ.
fn differences(left: &Value, right: &Value, pointer: &str, found: &mut Vec<String>, limit: usize) {
    if found.len() >= limit || left == right {
        return;
    }

    match (left, right) {
        (Value::Object(left), Value::Object(right)) => {
            let keys: BTreeSet<&String> = left.keys().chain(right.keys()).collect();
            for key in keys {
                let pointer = format!("{pointer}/{}", key.replace('~', "~0").replace('/', "~1"));
                match (left.get(key), right.get(key)) {
                    (Some(left), Some(right)) => {
                        differences(left, right, &pointer, found, limit);
                    }
                    _ => found.push(pointer),
                }
            }
        }
        (Value::Array(left), Value::Array(right)) if left.len() == right.len() => {
            for (index, (left, right)) in left.iter().zip(right).enumerate() {
                differences(left, right, &format!("{pointer}/{index}"), found, limit);
            }
        }
        _ => found.push(pointer.to_owned()),
    }
}

/// The committed `docs/api/openapi.yaml` holds the document the handlers generate. Compared as
/// parsed structures, so key order and YAML style do not matter.
#[test]
fn document_matches_committed() {
    let committed = std::fs::read_to_string(DOCUMENT_PATH).expect("read docs/api/openapi.yaml");
    let committed: Value = yaml_serde::from_str(&committed).expect("parse docs/api/openapi.yaml");
    let generated = document();

    let mut found = Vec::new();
    differences(&generated, &committed, "", &mut found, 20);
    assert!(
        found.is_empty(),
        "docs/api/openapi.yaml differs from the handlers at {found:#?}; \
         regenerate it with `cargo test -p wastebin -- --ignored write_openapi_document`"
    );
}

/// Writes `docs/api/openapi.yaml` from the handlers.
#[test]
#[ignore = "writes docs/api/openapi.yaml; run on purpose after changing the contract"]
fn write_openapi_document() {
    std::fs::write(DOCUMENT_PATH, document_yaml()).expect("write docs/api/openapi.yaml");
}

#[test]
fn differences_reports_a_planted_change() {
    let mut planted = document();
    planted["paths"]["/v1"]["get"]["x-phase"] = Value::from("P2");
    planted["components"]["schemas"]
        .as_object_mut()
        .expect("schemas")
        .remove("Link");

    let mut found = Vec::new();
    differences(&document(), &planted, "", &mut found, 20);
    assert_eq!(
        found,
        ["/components/schemas/Link", "/paths/~1v1/get/x-phase"]
    );
}

/// Every `(method, path)` of a document.
fn operations(document: &utoipa::openapi::OpenApi) -> BTreeSet<(String, String)> {
    let value = serde_json::to_value(document).expect("serialisable document");
    let mut operations = BTreeSet::new();

    for (path, item) in value["paths"].as_object().expect("paths") {
        for method in item.as_object().expect("path item").keys() {
            operations.insert((method.clone(), path.clone()));
        }
    }

    operations
}

/// The router serves exactly the operations annotated on handlers, and none declared in
/// `pending`. `GET /` is the new paste page, routed outside `/v1`.
#[test]
fn router_matches_document() {
    let routed: BTreeSet<(String, String)> = ROUTES
        .iter()
        .map(|(method, path)| ((*method).to_owned(), (*path).to_owned()))
        .chain([("get".to_owned(), "/".to_owned())])
        .collect();
    let annotated = operations(&Routed::openapi());
    let pending = operations(&Pending::openapi());

    assert_eq!(routed, annotated, "routes and annotated handlers differ");
    assert!(
        routed.is_disjoint(&pending),
        "pending declarations that are routed: {:?}",
        routed.intersection(&pending).collect::<Vec<_>>()
    );

    let document = document();
    for (method, path) in &annotated {
        assert_eq!(
            document["paths"][path][method]["x-phase"], "P1",
            "{method} {path}"
        );
    }
}
