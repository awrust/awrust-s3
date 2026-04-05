use awrust_s3_domain::{MemoryStore, PutObject, Store};
use std::collections::HashMap;

fn put(store: &MemoryStore, bucket: &str, key: &str, bytes: &[u8]) {
    store
        .put_object(
            bucket,
            key,
            PutObject {
                bytes: bytes.to_vec(),
                content_type: "application/octet-stream".to_string(),
                metadata: HashMap::new(),
            },
        )
        .unwrap();
}

#[test]
fn create_bucket() {
    let store = MemoryStore::new();
    store.create_bucket("bucket").unwrap();
    assert!(store.bucket_exists("bucket"));
}

#[test]
fn list_buckets() {
    let store = MemoryStore::new();
    store.create_bucket("bravo").unwrap();
    store.create_bucket("alpha").unwrap();
    assert_eq!(store.list_buckets(), vec!["alpha", "bravo"]);
}

#[test]
fn put_get_object() {
    let store = MemoryStore::new();
    store.create_bucket("bucket").unwrap();
    put(&store, "bucket", "key", b"payload");

    let obj = store.get_object("bucket", "key").unwrap();
    assert_eq!(obj.bytes, b"payload");
    assert_eq!(obj.meta.size, 7);
    assert!(!obj.meta.etag.is_empty());
}

#[test]
fn head_object() {
    let store = MemoryStore::new();
    store.create_bucket("bucket").unwrap();
    put(&store, "bucket", "key", b"payload");

    let meta = store.head_object("bucket", "key").unwrap();
    assert_eq!(meta.size, 7);
    assert_eq!(meta.content_type, "application/octet-stream");
}

#[test]
fn put_with_metadata() {
    let store = MemoryStore::new();
    store.create_bucket("bucket").unwrap();
    store
        .put_object(
            "bucket",
            "key",
            PutObject {
                bytes: b"data".to_vec(),
                content_type: "text/plain".to_string(),
                metadata: HashMap::from([("color".to_string(), "blue".to_string())]),
            },
        )
        .unwrap();

    let meta = store.head_object("bucket", "key").unwrap();
    assert_eq!(meta.content_type, "text/plain");
    assert_eq!(meta.metadata.get("color").unwrap(), "blue");
}

#[test]
fn delete_object() {
    let store = MemoryStore::new();
    store.create_bucket("bucket").unwrap();
    put(&store, "bucket", "key", b"payload");

    store.delete_object("bucket", "key").unwrap();
    assert!(store.get_object("bucket", "key").is_err());
}

#[test]
fn list_with_prefix() {
    let store = MemoryStore::new();
    store.create_bucket("bucket").unwrap();
    put(&store, "bucket", "a/1.txt", b"one");
    put(&store, "bucket", "a/2.txt", b"two");
    put(&store, "bucket", "b/1.txt", b"three");

    let listed = store.list_objects("bucket", Some("a/")).unwrap();
    let keys: Vec<&str> = listed.iter().map(|o| o.key.as_str()).collect();
    assert_eq!(keys, vec!["a/1.txt", "a/2.txt"]);
}
