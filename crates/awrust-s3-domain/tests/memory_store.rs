use awrust_s3_domain::{ListObjectsParams, MemoryStore, PutObject, Store, StoreError};
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
                delimiter: None,
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
                delimiter: None,
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
                delimiter: None,
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
                delimiter: None,
                continuation_token: page2.next_continuation_token,
                max_keys: 2,
            },
        )
        .unwrap();
    assert_eq!(page3.objects.len(), 1);
    assert!(!page3.is_truncated);
}

#[test]
fn delimiter_groups_nested_keys() {
    let store = MemoryStore::new();
    store.create_bucket("bucket").unwrap();
    put(&store, "bucket", "a.txt", b"one");
    put(&store, "bucket", "dir/b.txt", b"two");
    put(&store, "bucket", "dir/c.txt", b"three");
    put(&store, "bucket", "dir/sub/d.txt", b"four");

    let page = store
        .list_objects(
            "bucket",
            &ListObjectsParams {
                prefix: None,
                delimiter: Some("/".to_string()),
                continuation_token: None,
                max_keys: 1000,
            },
        )
        .unwrap();

    let keys: Vec<&str> = page.objects.iter().map(|o| o.key.as_str()).collect();
    assert_eq!(keys, vec!["a.txt"]);
    assert_eq!(page.common_prefixes, vec!["dir/"]);
    assert!(!page.is_truncated);
}

#[test]
fn delimiter_with_prefix() {
    let store = MemoryStore::new();
    store.create_bucket("bucket").unwrap();
    put(&store, "bucket", "photos/a.jpg", b"one");
    put(&store, "bucket", "photos/vacation/b.jpg", b"two");
    put(&store, "bucket", "photos/vacation/c.jpg", b"three");

    let page = store
        .list_objects(
            "bucket",
            &ListObjectsParams {
                prefix: Some("photos/".to_string()),
                delimiter: Some("/".to_string()),
                continuation_token: None,
                max_keys: 1000,
            },
        )
        .unwrap();

    let keys: Vec<&str> = page.objects.iter().map(|o| o.key.as_str()).collect();
    assert_eq!(keys, vec!["photos/a.jpg"]);
    assert_eq!(page.common_prefixes, vec!["photos/vacation/"]);
}

#[test]
fn delimiter_without_matches_returns_all_as_contents() {
    let store = MemoryStore::new();
    store.create_bucket("bucket").unwrap();
    put(&store, "bucket", "a.txt", b"one");
    put(&store, "bucket", "b.txt", b"two");

    let page = store
        .list_objects(
            "bucket",
            &ListObjectsParams {
                prefix: None,
                delimiter: Some("/".to_string()),
                continuation_token: None,
                max_keys: 1000,
            },
        )
        .unwrap();

    let keys: Vec<&str> = page.objects.iter().map(|o| o.key.as_str()).collect();
    assert_eq!(keys, vec!["a.txt", "b.txt"]);
    assert!(page.common_prefixes.is_empty());
}

#[test]
fn put_and_get_tagging() {
    let store = MemoryStore::new();
    store.create_bucket("bucket").unwrap();
    put(&store, "bucket", "key", b"data");

    let tags = HashMap::from([
        ("env".to_string(), "prod".to_string()),
        ("team".to_string(), "data".to_string()),
    ]);
    store
        .put_object_tagging("bucket", "key", tags.clone())
        .unwrap();

    let got = store.get_object_tagging("bucket", "key").unwrap();
    assert_eq!(got, tags);
}

#[test]
fn get_tagging_empty_by_default() {
    let store = MemoryStore::new();
    store.create_bucket("bucket").unwrap();
    put(&store, "bucket", "key", b"data");

    let tags = store.get_object_tagging("bucket", "key").unwrap();
    assert!(tags.is_empty());
}

#[test]
fn delete_tagging() {
    let store = MemoryStore::new();
    store.create_bucket("bucket").unwrap();
    put(&store, "bucket", "key", b"data");

    store
        .put_object_tagging(
            "bucket",
            "key",
            HashMap::from([("env".to_string(), "prod".to_string())]),
        )
        .unwrap();
    store.delete_object_tagging("bucket", "key").unwrap();

    let tags = store.get_object_tagging("bucket", "key").unwrap();
    assert!(tags.is_empty());
}

#[test]
fn put_object_clears_tags() {
    let store = MemoryStore::new();
    store.create_bucket("bucket").unwrap();
    put(&store, "bucket", "key", b"v1");

    store
        .put_object_tagging(
            "bucket",
            "key",
            HashMap::from([("env".to_string(), "prod".to_string())]),
        )
        .unwrap();

    put(&store, "bucket", "key", b"v2");

    let tags = store.get_object_tagging("bucket", "key").unwrap();
    assert!(tags.is_empty());
}

#[test]
fn tagging_on_missing_object() {
    let store = MemoryStore::new();
    store.create_bucket("bucket").unwrap();

    assert!(matches!(
        store.get_object_tagging("bucket", "nope"),
        Err(StoreError::ObjectNotFound { .. })
    ));
    assert!(matches!(
        store.put_object_tagging("bucket", "nope", HashMap::new()),
        Err(StoreError::ObjectNotFound { .. })
    ));
    assert!(matches!(
        store.delete_object_tagging("bucket", "nope"),
        Err(StoreError::ObjectNotFound { .. })
    ));
}

#[test]
fn tagging_on_missing_bucket() {
    let store = MemoryStore::new();

    assert!(matches!(
        store.get_object_tagging("nope", "key"),
        Err(StoreError::BucketNotFound(_))
    ));
}
