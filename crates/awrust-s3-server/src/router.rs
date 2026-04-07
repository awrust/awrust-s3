use awrust_s3_domain::Store;
use axum::extract::{DefaultBodyLimit, State};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, put};
use axum::{Json, Router, extract::Request};
use serde::Serialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::{Level, info};
use uuid::Uuid;

use crate::handlers;
use crate::vhost::VhostService;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

pub(crate) fn build(store: Arc<dyn Store>) -> (VhostService<Router>, SocketAddr) {
    let base_domain: Arc<String> = Arc::new(
        std::env::var("AWRUST_S3_BASE_DOMAIN").unwrap_or_else(|_| "localhost".to_string()),
    );
    info!(%base_domain, "virtual-host base domain");

    let app = Router::new()
        .route("/health", get(health))
        .route(
            "/:bucket",
            put(handlers::create_bucket)
                .head(handlers::head_bucket)
                .delete(handlers::delete_bucket)
                .get(handlers::get_bucket)
                .post(handlers::post_bucket),
        )
        .route(
            "/:bucket/*key",
            put(handlers::put_object_or_part)
                .get(handlers::get_object)
                .head(handlers::head_object)
                .post(handlers::post_object)
                .delete(handlers::delete_object_or_abort),
        )
        .fallback(
            |State(store): State<Arc<dyn Store>>, req: Request| async move {
                if req.uri().path() == "/" {
                    handlers::list_buckets(State(store)).await
                } else {
                    axum::http::StatusCode::NOT_FOUND.into_response()
                }
            },
        )
        .with_state(store)
        .layer(DefaultBodyLimit::max(5 * 1024 * 1024 * 1024))
        .layer(middleware::from_fn(request_id))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .layer(CorsLayer::very_permissive());

    let service = VhostService::new(app, base_domain);

    let addr: SocketAddr = std::env::var("AWRUST_S3_LISTEN_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:4566".to_string())
        .parse()
        .expect("valid listen addr");

    (service, addr)
}

async fn request_id(req: Request, next: Next) -> Response {
    let id = Uuid::new_v4().to_string();
    let mut resp = next.run(req).await;
    resp.headers_mut()
        .insert("x-amz-request-id", id.parse().expect("valid header"));
    resp
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}
