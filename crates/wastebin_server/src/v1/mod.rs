//! The `/v1` JSON API (plan section 四). `docs/api/openapi.yaml` is its contract, generated from
//! the annotations in this module by the `contract` tests.

mod auth;
#[cfg(test)]
mod contract;
mod error;
mod model;
mod pastes;
#[cfg(test)]
mod tests;

use axum::Router;
use axum::routing::MethodRouter;

use crate::AppState;

pub(crate) use error::{envelope, is_v1};

/// Declares the `/v1` routes once, for the router and for the test that compares the router with
/// the generated document.
macro_rules! v1_routes {
    ($($path:literal { $($method:ident => $handler:path),+ $(,)? })+) => {
        /// Router of every built `/v1` operation.
        pub(crate) fn router() -> Router<AppState> {
            Router::new()$(.route($path, MethodRouter::new()$(.$method($handler))+))+
        }

        /// Every `(method, path)` the router serves.
        #[cfg(test)]
        pub(crate) const ROUTES: &[(&str, &str)] = &[$($((stringify!($method), $path)),+),+];
    };
}

v1_routes! {
    "/v1" { get => pastes::get_version }
    "/v1/pastes" { post => pastes::create }
    "/v1/pastes/{id}" { get => pastes::show, delete => pastes::delete }
    "/v1/pastes/{id}/content" { get => pastes::get_content }
}
