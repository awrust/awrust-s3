use tracing_subscriber::{EnvFilter, fmt};

pub(crate) fn init() {
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
