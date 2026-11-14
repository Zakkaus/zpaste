//! `GET /v1/pastes`: the caller's pastes, one page at a time.

use axum::Json;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
#[cfg(test)]
use serde::Serialize;
#[cfg(test)]
use utoipa::{IntoParams, ToSchema};

use wastebin_core::id::Id;

use crate::AppState;
use crate::v1::auth;
#[cfg(test)]
use crate::v1::contract::responses::{BadRequest, Unauthorized};
use crate::v1::error::ApiError;
#[cfg(test)]
use crate::v1::model::PasteId;
use crate::v1::model::PasteListDocument;

const DEFAULT_LIST_LIMIT: usize = 20;
const MAX_LIST_LIMIT: usize = 100;

/// The query of `listPastes` as sent; [`list`] checks each value, so every malformed one is the
/// same 400 in the envelope.
#[derive(Deserialize)]
pub(crate) struct ListParams {
    owner: Option<String>,
    limit: Option<String>,
    marker: Option<String>,
}

// The documented values of `owner`; `ListParams` reads the query.
#[cfg(test)]
#[allow(dead_code, reason = "a schema of the generated contract")]
#[derive(Serialize, ToSchema)]
enum OwnerFilter {
    #[serde(rename = "me")]
    Me,
}

// The documented query of `listPastes`; `ListParams` reads it.
#[cfg(test)]
#[allow(dead_code, reason = "a schema of the generated contract")]
#[derive(Serialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct ListQuery {
    /// Only `me` is defined.
    #[param(inline)]
    owner: OwnerFilter,
    #[param(minimum = 1, maximum = 100, default = 20)]
    limit: Option<u32>,
    /// Id of the last paste on the previous page. A marker that is malformed,
    /// deleted, expired or owned by another identity gives the same 400.
    marker: Option<PasteId>,
}

/// List the caller's pastes
///
/// Filters by identity and expiry first, then pages. Order is newest first by
/// `(created_at, id)`; pastes from before the `created_at` column sort last.
/// `pastes_links` holds only a `next` link, which keeps `owner=me` and `limit`.
/// With the cookie, `me` is every identity the cookie holds.
#[cfg_attr(test, utoipa::path(
    get,
    path = "/v1/pastes",
    tag = "pastes",
    operation_id = "listPastes",
    extensions(("x-phase" = json!("P1"))),
    security(("ownerToken" = []), ("ownerCookie" = [])),
    params(ListQuery),
    responses(
        (status = 200, description = "One page of pastes.", body = PasteListDocument),
        (status = 400, response = BadRequest),
        (status = 401, response = Unauthorized),
    ),
))]
pub(crate) async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
    query: Result<Query<ListParams>, QueryRejection>,
) -> Result<Response, ApiError> {
    let caller = auth::caller(&headers, &state.key, &state.page.base_url, false)?;
    let uids = caller.uids();
    if uids.is_empty() {
        return Err(ApiError::unauthorized(
            "an owner token or the uid cookie is required",
        ));
    }

    let Query(params) = query.map_err(|rejection| ApiError::bad_request(rejection.body_text()))?;
    if params.owner.as_deref() != Some("me") {
        return Err(ApiError::bad_request("owner must be me"));
    }
    let limit = match params.limit.as_deref() {
        None => DEFAULT_LIST_LIMIT,
        Some(limit) => limit
            .parse::<usize>()
            .ok()
            .filter(|limit| (1..=MAX_LIST_LIMIT).contains(limit))
            .ok_or_else(|| ApiError::bad_request("limit must be an integer from 1 to 100"))?,
    };
    let marker = params
        .marker
        .as_deref()
        .map(|marker| marker.parse::<Id>().map_err(|_| invalid_marker()))
        .transpose()?;

    let page = match state.db.list_for(uids, marker, limit).await {
        Ok(page) => page,
        Err(wastebin_core::db::Error::InvalidMarker) => return Err(invalid_marker()),
        Err(err) => return Err(err.into()),
    };
    let next = page
        .more
        .then(|| page.heads.last())
        .flatten()
        .map(|last| next_query(limit, last.id));

    Ok(Json(PasteListDocument::new(
        &page.heads,
        &state.page.base_url,
        next,
    ))
    .into_response())
}

/// The query of the page after `marker`. Ids may hold `+`, which a query decodes as a space unless
/// it is encoded.
fn next_query(limit: usize, marker: Id) -> String {
    url::form_urlencoded::Serializer::new(String::new())
        .append_pair("owner", "me")
        .append_pair("limit", &limit.to_string())
        .append_pair("marker", &marker.to_string())
        .finish()
}

/// One answer for every marker that is not an unexpired paste of the caller, so a marker reveals
/// nothing about an id (decision D6).
fn invalid_marker() -> ApiError {
    ApiError::bad_request("marker is not a paste of this list")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_query_survives_a_plus_in_the_marker() -> Result<(), Box<dyn std::error::Error>> {
        let marker: Id = "ab+c-1".parse()?;
        let id = marker.to_string();
        assert!(id.contains('+'), "{id}");
        let query = next_query(2, marker);
        let decoded = url::form_urlencoded::parse(query.as_bytes())
            .into_owned()
            .collect::<Vec<_>>();
        assert_eq!(
            decoded,
            [
                (String::from("owner"), String::from("me")),
                (String::from("limit"), String::from("2")),
                (String::from("marker"), id),
            ]
        );
        Ok(())
    }
}
