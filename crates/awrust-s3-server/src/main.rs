use axum::{Json, Router, routing::get};
use serde::Serialize;
use std::net::SocketAddr;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::{info, Level};
use tracing_subscriber::{EnvFilter, fmt};

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[tokio::main]
async fn main() {
    init_tracing();

    let app = Router::new()
        .route("/health", get(health))
        .layer(TraceLayer::new_for_http()
            .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
            .on_response(DefaultOnResponse::new().level(Level::INFO)));

    let addr: SocketAddr = "0.0.0.0:4566".parse().expect("valid listen addr");
    info!(service="s3", %addr, "awrust-s3 server starting");

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
