mod error;
mod handlers;
mod vhost;
mod xml;

use awrust_s3_domain::{FsStore, MemoryStore, Store};
use axum::extract::{DefaultBodyLimit, State};
use axum::middleware::{self, Next};
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::{get, put};
use axum::{Json, Router, ServiceExt, extract::Request};
use serde::Serialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::{Level, info};
use tracing_subscriber::{EnvFilter, fmt};
use uuid::Uuid;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[tokio::main]
async fn main() {
    init_tracing();

    let store_type = std::env::var("AWRUST_S3_STORE").unwrap_or_else(|_| "memory".to_string());
    let store: Arc<dyn Store> = match store_type.as_str() {
        "fs" => {
            let data_dir =
                std::env::var("AWRUST_S3_DATA_DIR").unwrap_or_else(|_| "/data".to_string());
            info!(backend = "fs", %data_dir, "using filesystem store");
            Arc::new(FsStore::new(data_dir))
        }
        _ => {
            info!(backend = "memory", "using in-memory store");
            Arc::new(MemoryStore::new())
        }
    };

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
            |State(store): axum::extract::State<Arc<dyn Store>>, req: Request| async move {
                if req.uri().path() == "/" {
                    handlers::list_buckets(axum::extract::State(store)).await
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
        );

    let app = vhost::VhostService::new(app, base_domain);

    let addr: SocketAddr = std::env::var("AWRUST_S3_LISTEN_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:4566".to_string())
        .parse()
        .expect("valid listen addr");
    info!(service = "s3", %addr, "awrust-s3 server starting");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind listen addr");

    axum::serve(listener, app.into_make_service())
        .await
        .expect("server error");
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

fn init_tracing() {
    let filter =
        EnvFilter::try_new(std::env::var("AWRUST_LOG").unwrap_or_else(|_| "info".to_string()))
            .expect("valid log filter");

    fmt()
        .json()
        .with_env_filter(filter)
        .with_current_span(true)
        .with_span_list(true)
        .init();
}
