//! Middleware layering regression tests.
//!
//! `jwt_middleware` must satisfy axum's `Service` bounds when applied via
//! `from_fn_with_state`, which requires its future to be `Send`. This failed
//! to compile while `verify_jwt_from_header` boxed errors without `Send`.

#![cfg(feature = "oauth")]

use axum::{Router, routing::post};
use std::sync::Arc;
use vyna_hl7_http::auth::JwtConfig;
use vyna_hl7_http::middleware::{JwtAuthState, jwt_middleware};

#[test]
fn jwt_middleware_layers_onto_router() {
    async fn handler() -> &'static str {
        "OK"
    }
    let config = JwtConfig::new_hs256(b"secret");
    let auth_state = JwtAuthState::new(Arc::new(config));
    let app: Router = Router::new()
        .route("/", post(handler))
        .route_layer(axum::middleware::from_fn_with_state(
            auth_state.clone(),
            jwt_middleware,
        ))
        .with_state(auth_state);
    let _ = app;
}
