use awrust_s3_domain::{MemoryStore, Store};

#[test]
fn create_bucket() {
    let store = MemoryStore::new();
    store.create_bucket("bucket").unwrap();
    assert!(store.bucket_exists("bucket"));
}

#[test]
fn put_get_object() {
    let store = MemoryStore::new();
    store.create_bucket("bucket").unwrap();
    store
        .put_object("bucket", "key", b"payload".to_vec())
        .unwrap();

    let bytes = store.get_object("bucket", "key").unwrap();
    assert_eq!(bytes, b"payload".to_vec());
}

#[test]
fn delete_object() {
    let store = MemoryStore::new();
    store.create_bucket("bucket").unwrap();
    store
        .put_object("bucket", "key", b"payload".to_vec())
        .unwrap();

    store.delete_object("bucket", "key").unwrap();
    assert!(store.get_object("bucket", "key").is_err());
}

#[test]
fn list_with_prefix() {
    let store = MemoryStore::new();
    store.create_bucket("bucket").unwrap();
    store
        .put_object("bucket", "a/1.txt", b"one".to_vec())
        .unwrap();
    store
        .put_object("bucket", "a/2.txt", b"two".to_vec())
        .unwrap();
    store
        .put_object("bucket", "b/1.txt", b"three".to_vec())
        .unwrap();

    let listed = store.list_objects("bucket", Some("a/")).unwrap();
    let keys: Vec<&str> = listed.iter().map(|o| o.key.as_str()).collect();
    assert_eq!(keys, vec!["a/1.txt", "a/2.txt"]);
}
