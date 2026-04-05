mod error;
mod handlers;
mod xml;

use awrust_s3_domain::{MemoryStore, Store};
use axum::routing::{get, put};
use axum::{Json, Router};
use serde::Serialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::{Level, info};
use tracing_subscriber::{EnvFilter, fmt};

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[tokio::main]
async fn main() {
    init_tracing();

    let store: Arc<dyn Store> = Arc::new(MemoryStore::new());

    let app = Router::new()
        .route("/health", get(health))
        .route(
            "/:bucket",
            put(handlers::create_bucket)
                .head(handlers::head_bucket)
                .delete(handlers::delete_bucket)
                .get(handlers::list_objects),
        )
        .route(
            "/:bucket/*key",
            put(handlers::put_object)
                .get(handlers::get_object)
                .head(handlers::head_object)
                .delete(handlers::delete_object),
        )
        .with_state(store)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        );

    let addr: SocketAddr = "0.0.0.0:4566".parse().expect("valid listen addr");
    info!(service = "s3", %addr, "awrust-s3 server starting");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind listen addr");

    axum::serve(listener, app).await.expect("server error");
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(Level::INFO.to_string()));

    fmt()
        .json()
        .with_env_filter(filter)
        .with_current_span(true)
        .with_span_list(true)
        .init();
}
