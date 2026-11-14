//! Shared components (`securitySchemes`, `parameters`, `headers`, `responses`) and the `$ref`s
//! to them that utoipa's path macro cannot write.

use serde_json::{Value, json};
use utoipa::Modify;
use utoipa::openapi::path::Operation;
use utoipa::openapi::{Header, OpenApi as Document, Ref, RefOr};

use super::responses;
use crate::v1::is_v1;

fn component<T: serde::de::DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).expect("valid component")
}

fn reference(kind: &str, name: &str) -> RefOr<Header> {
    RefOr::Ref(Ref::new(format!("#/components/{kind}/{name}")))
}

/// A response header; `required` travels as `x-required` until [`super::patch`], because utoipa's
/// `Header` has no `required` field.
fn header(description: &str, schema: Value, required: bool) -> RefOr<Header> {
    RefOr::T(component(json!({
        "description": description,
        "schema": schema,
        "x-required": required,
    })))
}

fn owned(
    headers: impl IntoIterator<Item = (&'static str, RefOr<Header>)>,
) -> impl Iterator<Item = (String, RefOr<Header>)> {
    headers
        .into_iter()
        .map(|(name, header)| (name.to_owned(), header))
}

/// Shared components and the `$ref`s that utoipa's path macro cannot write.
pub(super) struct Contract;

impl Contract {
    fn security_schemes() -> Value {
        json!({
            "ownerToken": {
                "type": "http",
                "scheme": "bearer",
                "description": "`Authorization: Bearer <owner_token>`. The token is the server's signature\nover an identity's uid. Browsers never send it on their own, so Bearer\nrequests skip the `Origin` check. When present it is the only credential\nconsidered; a token that fails verification is 401 with no fallback.\n",
            },
            "ownerCookie": {
                "type": "apiKey",
                "in": "cookie",
                "name": "uid",
                "description": "The web app's signed cookie, in upstream's format (several identities;\nthe first one creates), set `HttpOnly; Secure; SameSite=Lax; Path=/`.\nConsidered only when no `Authorization` header is sent. Because browsers\nsend it on their own, a write that carries it must send `Origin` equal to\nthe configured public URL (scheme, host and port); a missing, `null` or\nforeign `Origin` is 403. That includes a `POST .../content` that carries\nit, although that operation needs no credentials.\n",
            },
        })
    }

    fn parameters() -> Value {
        json!({
            "RequestId": {
                "name": "X-Request-Id",
                "in": "header",
                "required": false,
                "description": "Kept when it matches `[A-Za-z0-9-]{1,64}`; any other value is replaced by\none the server makes, never rejected. Echoed on every response.\n",
                "schema": {"type": "string"},
            },
            "PasteIdPath": {
                "name": "id",
                "in": "path",
                "required": true,
                "description": "Paste id without extension.",
                "schema": {"$ref": "#/components/schemas/PasteId"},
            },
            "SlugPath": {
                "name": "slug",
                "in": "path",
                "required": true,
                "schema": {"$ref": "#/components/schemas/Slug"},
            },
            "EntryNamePath": {
                "name": "name",
                "in": "path",
                "required": true,
                "description": "Entry name, unique within the workspace, percent-encoded in the path.\nThe server normalises it to NFC before comparing or storing. Any Unicode\nexcept `/` and control characters; not `.` or `..`; 1 to 128 bytes of\nUTF-8 (the schema can only bound characters), so Chinese names work.\n",
                "schema": {"$ref": "#/components/schemas/EntryName"},
            },
            "IfMatch": {
                "name": "If-Match",
                "in": "header",
                "required": false,
                "description": "RFC 9110 syntax: `*` or a comma-separated list of entity tags. The\ncomparison is strong: a weak tag (`W/\"...\"`) never matches. A list matches\nwhen any strong tag equals the current `\"<version_id>\"`; `*` matches when\na current version exists. No match is 412. A value that does not parse is\n400. Without the header the write overwrites whatever is current.\n",
                "schema": {"$ref": "#/components/schemas/IfMatchList"},
            },
            "PastePasswordHeader": {
                "name": "X-Paste-Password",
                "in": "header",
                "required": false,
                "description": "Password of an encrypted paste, as base64url of its UTF-8 bytes. Required\nwhen the paste is encrypted; never logged.\n",
                "schema": {"type": "string", "pattern": "^[A-Za-z0-9_-]+={0,2}$"},
            },
        })
    }

    fn headers() -> Vec<(&'static str, RefOr<Header>)> {
        vec![
            (
                "RequestId",
                header(
                    "The request id the server used.",
                    json!({"type": "string", "pattern": "^[A-Za-z0-9-]{1,64}$"}),
                    true,
                ),
            ),
            (
                "NoStore",
                header(
                    "Always `no-store` on `/v1`.",
                    json!({"type": "string", "enum": ["no-store"]}),
                    true,
                ),
            ),
            (
                "ETag",
                header(
                    "The current version, `\"<version_id>\"`.",
                    json!({"$ref": "#/components/schemas/EntityTag"}),
                    true,
                ),
            ),
            (
                "PasteLocation",
                header(
                    "Path of the paste, `/v1/pastes/{id}`.",
                    json!({"type": "string", "example": "/v1/pastes/abc123"}),
                    true,
                ),
            ),
            (
                "RetryAfter",
                header(
                    "Seconds to wait before retrying.",
                    json!({"type": "integer", "minimum": 0}),
                    true,
                ),
            ),
            (
                "BearerChallenge",
                header(
                    "Challenge for the owner token, `Bearer realm=\"zpaste\"`. When the 401 comes\nfrom a missing or failed `X-Paste-Password` on an edit, the password\nchallenge (`PasswordChallenge`) instead.\n",
                    json!({"type": "string"}),
                    true,
                ),
            ),
            (
                "PasswordChallenge",
                header(
                    "Challenge for the paste password. `Paste-Password` is a custom\nauthentication scheme of this API, not registered with IANA; clients\nrecognise it to prompt for the password.\n",
                    json!({"type": "string", "enum": ["Paste-Password realm=\"zpaste\""]}),
                    true,
                ),
            ),
        ]
    }

    /// Headers of a shared response beyond `X-Request-Id` and `Cache-Control`.
    fn response_headers(name: &str) -> Vec<(&'static str, RefOr<Header>)> {
        match name {
            "Unauthorized" => vec![("WWW-Authenticate", reference("headers", "BearerChallenge"))],
            "WrongPassword" => {
                vec![(
                    "WWW-Authenticate",
                    reference("headers", "PasswordChallenge"),
                )]
            }
            "PreconditionFailed" => vec![("ETag", reference("headers", "ETag"))],
            "EntryPreconditionFailed" => vec![(
                "ETag",
                header(
                    "The current version, `\"<version_id>\"`, when there is one.",
                    json!({"$ref": "#/components/schemas/EntityTag"}),
                    false,
                ),
            )],
            "TooManyRequests" => vec![("Retry-After", reference("headers", "RetryAfter"))],
            _ => vec![],
        }
    }

    /// Headers of an operation's success response beyond `X-Request-Id` and `Cache-Control`.
    fn success_headers(operation_id: &str, status: &str) -> Vec<(&'static str, RefOr<Header>)> {
        let etag = || ("ETag", reference("headers", "ETag"));
        let location = || ("Location", reference("headers", "PasteLocation"));

        match (operation_id, status) {
            ("createPaste", "201") => vec![
                location(),
                etag(),
                (
                    "Set-Cookie",
                    header(
                        "Only on a browser create that minted a new identity. Carries the signed `uid` cookie.",
                        json!({"type": "string"}),
                        false,
                    ),
                ),
            ],
            (
                "showPaste" | "updatePaste" | "getContent" | "readContent" | "replaceContent"
                | "readVersionContent" | "putWorkspaceEntry",
                "200",
            ) => vec![etag()],
            ("createWorkspace", "201") => vec![(
                "Location",
                header(
                    "Path of the new workspace, `/v1/workspaces/{slug}`.",
                    json!({"type": "string", "example": "/v1/workspaces/v2raya"}),
                    true,
                ),
            )],
            ("addWorkspaceEntry", "201") => vec![location()],
            ("putWorkspaceEntry", "201") => vec![etag(), location()],
            _ => vec![],
        }
    }

    /// Shared parameters of an operation: path parameters by placeholder, the request id, and
    /// the edit headers.
    fn parameters_of(
        path: &str,
        operation_id: &str,
    ) -> Vec<RefOr<utoipa::openapi::path::Parameter>> {
        let mut names = Vec::new();
        for (placeholder, name) in [
            ("{id}", "PasteIdPath"),
            ("{slug}", "SlugPath"),
            ("{name}", "EntryNamePath"),
        ] {
            if path.contains(placeholder) {
                names.push(name);
            }
        }

        names.push("RequestId");

        if matches!(
            operation_id,
            "updatePaste" | "replaceContent" | "putWorkspaceEntry"
        ) {
            names.extend(["IfMatch", "PastePasswordHeader"]);
        }

        names
            .into_iter()
            .map(|name| RefOr::Ref(Ref::new(format!("#/components/parameters/{name}"))))
            .collect()
    }

    fn standard_headers() -> [(&'static str, RefOr<Header>); 2] {
        [
            ("X-Request-Id", reference("headers", "RequestId")),
            ("Cache-Control", reference("headers", "NoStore")),
        ]
    }

    fn operation(path: &str, operation: &mut Operation) {
        let operation_id = operation.operation_id.clone().unwrap_or_default();

        let mut parameters = Self::parameters_of(path, &operation_id);
        parameters.extend(operation.parameters.take().unwrap_or_default());
        operation.parameters = Some(parameters);

        for (status, response) in &mut operation.responses.responses {
            if let RefOr::T(response) = response {
                response.headers.extend(owned(Self::standard_headers()));
                response
                    .headers
                    .extend(owned(Self::success_headers(&operation_id, status)));
            }
        }

        operation.responses.responses.insert(
            "default".to_owned(),
            RefOr::Ref(Ref::new("#/components/responses/Error")),
        );
    }
}

impl Modify for Contract {
    fn modify(&self, openapi: &mut Document) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components
            .security_schemes
            .extend(component::<std::collections::BTreeMap<_, _>>(
                Self::security_schemes(),
            ));
        components
            .parameters
            .extend(component::<std::collections::BTreeMap<_, _>>(
                Self::parameters(),
            ));
        components.headers.extend(owned(Self::headers()));

        for (name, mut response) in responses::all() {
            if let RefOr::T(response) = &mut response {
                response.headers.extend(owned(Self::standard_headers()));
                response.headers.extend(owned(Self::response_headers(name)));
            }
            components.responses.insert(name.to_owned(), response);
        }

        for (path, item) in &mut openapi.paths.paths {
            if !is_v1(path) {
                continue;
            }

            for operation in [
                &mut item.get,
                &mut item.post,
                &mut item.put,
                &mut item.patch,
                &mut item.delete,
            ]
            .into_iter()
            .flatten()
            {
                Self::operation(path, operation);
            }
        }
    }
}
