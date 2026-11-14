//! Shared error responses (`components.responses`), referenced from `#[utoipa::path]` by marker
//! type. Their headers are added by [`super::Contract`].

use serde_json::{Value, json};
use utoipa::ToResponse;
use utoipa::openapi::{RefOr, Response};

fn error_response(description: &str, example: Option<Value>) -> RefOr<Response> {
    let mut media = json!({"schema": {"$ref": "#/components/schemas/ErrorDocument"}});
    if let Some(example) = example {
        media["example"] = json!({"error": example});
    }

    let response = json!({
        "description": description,
        "content": {"application/json": media},
    });

    RefOr::T(serde_json::from_value(response).expect("valid response object"))
}

macro_rules! error_responses {
    ($($name:ident: $description:expr $(, example = $example:expr)?;)+) => {
        $(
            #[allow(dead_code, reason = "a marker for `response = ...` in `#[utoipa::path]`")]
            pub(crate) struct $name;

            impl<'s> ToResponse<'s> for $name {
                fn response() -> (&'s str, RefOr<Response>) {
                    let example: Option<Value> = None $(.or(Some($example)))?;
                    (stringify!($name), error_response($description, example))
                }
            }
        )+

        /// Every shared response, by component name.
        pub(crate) fn all() -> Vec<(&'static str, RefOr<Response>)> {
            vec![$((stringify!($name), <$name as ToResponse>::response().1)),+]
        }
    };
}

error_responses! {
    Error: "Any other error, in the same shape (for example 405, 408, 415, 500, 503).";
    BadRequest: "The request is malformed or a field is invalid (`bad_request`). Body parse errors are 400, not 422.",
        example = json!({"code": 400, "type": "bad_request", "title": "Bad Request", "message": "invalid marker", "request_id": "req-1"});
    Unauthorized: "No credentials, or credentials that fail verification (`unauthorized`).\nOn an edit of an encrypted paste, also a missing or wrong\n`X-Paste-Password` (`wrong_password`); nothing is written.\n";
    WrongPassword: "The password is wrong (`wrong_password`).",
        example = json!({"code": 401, "type": "wrong_password", "title": "Unauthorized", "message": "wrong password", "request_id": "req-1"});
    Forbidden: "Valid credentials that do not own the resource, two resources of\ndifferent identities, or a cookie request without a matching `Origin`\n(`forbidden`).\n";
    NotFound: "Does not exist or has expired; the two are not told apart (`not_found`).";
    ReadPrecondition: "The read needs a password (`password_required`) or a burn confirmation\n(`confirmation_required`). Nothing is returned or consumed.\n",
        example = json!({"code": 409, "type": "confirmation_required", "title": "Conflict", "message": "confirm to read; the paste is deleted after reading", "request_id": "req-1"});
    BurnNotEditable: "A burn-after-reading paste cannot be edited (`burn_not_editable`).";
    BurnNoHistory: "A burn-after-reading paste has no history endpoints (`burn_no_history`). Nothing is consumed.";
    VersionReadConflict: "The paste is burn-after-reading (`burn_no_history`), or the version is\nencrypted and the body has no password (`password_required`). Nothing is\nreturned or consumed.\n";
    SlugTaken: "The slug is already taken (`slug_taken`).";
    EntryAddConflict: "The name is taken (`entry_name_taken`), the paste is already in a\nworkspace (`paste_in_workspace`), or the workspace is full\n(`workspace_full`).\n";
    EntryWriteConflict: "The entry's paste is burn-after-reading (`burn_not_editable`), or a create\nwould exceed the workspace's entry limit (`workspace_full`).\n";
    PreconditionFailed: "`If-Match` matches no current version (`version_conflict`). `ETag` carries the current version.",
        example = json!({"code": 412, "type": "version_conflict", "title": "Precondition Failed", "message": "content changed since version 3", "request_id": "req-1"});
    EntryPreconditionFailed: "`If-Match` matches no current version (`version_conflict`). `ETag`\ncarries the current version when the entry has one; it is absent when the\nname is new or its paste has expired.\n";
    PayloadTooLarge: "The body exceeds `WASTEBIN_MAX_BODY_SIZE` bytes (`payload_too_large`).\nHistory limits never cause it; they trim old versions instead.\n";
    TooManyRequests: "Every slot of the shared limit on concurrent password key derivations\nstayed taken while the request waited (`rate_limited`). Nothing was derived,\nwritten or consumed. Only requests that derive a key count.\n";
}
