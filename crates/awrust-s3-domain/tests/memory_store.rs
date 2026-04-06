use awrust_s3_domain::{ListObjectsParams, MemoryStore, PutObject, Store};
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
    let buckets = store.list_buckets();
    let names: Vec<&str> = buckets.iter().map(|b| b.name.as_str()).collect();
    assert_eq!(names, vec!["alpha", "bravo"]);
    assert!(buckets[0].created > 0);
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

    let page = store
        .list_objects(
            "bucket",
            &ListObjectsParams {
                prefix: Some("a/".to_string()),
                continuation_token: None,
                max_keys: 1000,
            },
        )
        .unwrap();
    let keys: Vec<&str> = page.objects.iter().map(|o| o.key.as_str()).collect();
    assert_eq!(keys, vec!["a/1.txt", "a/2.txt"]);
}

#[test]
fn multipart_upload() {
    let store = MemoryStore::new();
    store.create_bucket("bucket").unwrap();

    let upload_id = store
        .initiate_multipart("bucket", "big", "application/octet-stream", HashMap::new())
        .unwrap();
    let etag1 = store
        .upload_part("bucket", "big", &upload_id, 1, b"part1".to_vec())
        .unwrap();
    let etag2 = store
        .upload_part("bucket", "big", &upload_id, 2, b"part2".to_vec())
        .unwrap();
    store
        .complete_multipart("bucket", "big", &upload_id, &[(1, etag1), (2, etag2)])
        .unwrap();

    let obj = store.get_object("bucket", "big").unwrap();
    assert_eq!(obj.bytes, b"part1part2");
    assert!(obj.meta.etag.contains('-'));
}

#[test]
fn abort_multipart() {
    let store = MemoryStore::new();
    store.create_bucket("bucket").unwrap();

    let upload_id = store
        .initiate_multipart(
            "bucket",
            "aborted",
            "application/octet-stream",
            HashMap::new(),
        )
        .unwrap();
    store
        .upload_part("bucket", "aborted", &upload_id, 1, b"data".to_vec())
        .unwrap();
    store
        .abort_multipart("bucket", "aborted", &upload_id)
        .unwrap();

    assert!(store.get_object("bucket", "aborted").is_err());
}

#[test]
fn pagination() {
    let store = MemoryStore::new();
    store.create_bucket("bucket").unwrap();
    for i in 0..5 {
        put(&store, "bucket", &format!("key{i:02}"), b"data");
    }

    let page1 = store
        .list_objects(
            "bucket",
            &ListObjectsParams {
                prefix: None,
                continuation_token: None,
                max_keys: 2,
            },
        )
        .unwrap();
    assert_eq!(page1.objects.len(), 2);
    assert!(page1.is_truncated);
    assert!(page1.next_continuation_token.is_some());

    let page2 = store
        .list_objects(
            "bucket",
            &ListObjectsParams {
                prefix: None,
                continuation_token: page1.next_continuation_token,
                max_keys: 2,
            },
        )
        .unwrap();
    assert_eq!(page2.objects.len(), 2);
    assert!(page2.is_truncated);

    let page3 = store
        .list_objects(
            "bucket",
            &ListObjectsParams {
                prefix: None,
                continuation_token: page2.next_continuation_token,
                max_keys: 2,
            },
        )
        .unwrap();
    assert_eq!(page3.objects.len(), 1);
    assert!(!page3.is_truncated);
}
