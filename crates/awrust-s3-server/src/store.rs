use awrust_s3_domain::{FsStore, MemoryStore, Store};
use std::sync::Arc;
use tracing::info;

pub(crate) fn build() -> Arc<dyn Store> {
    let store_type = std::env::var("AWRUST_S3_STORE").unwrap_or_else(|_| "memory".to_string());
    match store_type.as_str() {
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
    }
}
