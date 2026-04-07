mod error;
mod handlers;
mod router;
mod store;
mod tracing_init;
mod vhost;
mod xml;

use axum::ServiceExt;
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_init::init();

    let store = store::build();
    let (app, addr) = router::build(store);

    info!(service = "s3", %addr, "awrust-s3 server starting");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind listen addr");

    axum::serve(listener, app.into_make_service())
        .await
        .expect("server error");
}
