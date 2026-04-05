use crate::{GetObject, ObjectMeta, ObjectSummary, PutObject, Result, Store, StoreError};
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct FsStore {
    root: PathBuf,
}

#[derive(Serialize, Deserialize)]
struct MetaFile {
    etag: String,
    content_type: String,
    metadata: HashMap<String, String>,
    last_modified: u64,
    size: u64,
}

fn encode_key(key: &str) -> String {
    key.replace('%', "%25")
        .replace('/', "%2F")
        .replace('\\', "%5C")
        .replace('\0', "%00")
}

fn decode_key(encoded: &str) -> String {
    encoded
        .replace("%2F", "/")
        .replace("%5C", "\\")
        .replace("%00", "\0")
        .replace("%25", "%")
}

impl FsStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        fs::create_dir_all(&root).expect("create data dir");
        Self { root }
    }

    fn bucket_path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    fn objects_dir(&self, bucket: &str) -> PathBuf {
        self.bucket_path(bucket).join("objects")
    }

    fn meta_dir(&self, bucket: &str) -> PathBuf {
        self.bucket_path(bucket).join("meta")
    }

    fn object_path(&self, bucket: &str, key: &str) -> PathBuf {
        self.objects_dir(bucket).join(encode_key(key))
    }

    fn meta_path(&self, bucket: &str, key: &str) -> PathBuf {
        self.meta_dir(bucket)
            .join(format!("{}.json", encode_key(key)))
    }

    fn require_bucket(&self, bucket: &str) -> Result<()> {
        if !self.bucket_path(bucket).is_dir() {
            return Err(StoreError::BucketNotFound(bucket.to_string()));
        }
        Ok(())
    }

    fn read_meta(&self, bucket: &str, key: &str) -> Result<MetaFile> {
        let path = self.meta_path(bucket, key);
        let data = fs::read(&path).map_err(|_| StoreError::ObjectNotFound {
            bucket: bucket.to_string(),
            key: key.to_string(),
        })?;
        serde_json::from_slice(&data).map_err(|_| StoreError::ObjectNotFound {
            bucket: bucket.to_string(),
            key: key.to_string(),
        })
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time after epoch")
        .as_secs()
}

impl Store for FsStore {
    fn create_bucket(&self, name: &str) -> Result<()> {
        let bp = self.bucket_path(name);
        fs::create_dir_all(self.objects_dir(name)).ok();
        fs::create_dir_all(self.meta_dir(name)).ok();
        let _ = bp;
        Ok(())
    }

    fn bucket_exists(&self, name: &str) -> bool {
        self.bucket_path(name).is_dir()
    }

    fn delete_bucket(&self, name: &str) -> Result<()> {
        self.require_bucket(name)?;

        let has_objects = fs::read_dir(self.objects_dir(name))
            .map(|mut d| d.next().is_some())
            .unwrap_or(false);

        if has_objects {
            return Err(StoreError::BucketNotEmpty(name.to_string()));
        }

        fs::remove_dir_all(self.bucket_path(name)).ok();
        Ok(())
    }

    fn list_buckets(&self) -> Vec<crate::BucketSummary> {
        let mut summaries: Vec<crate::BucketSummary> = fs::read_dir(&self.root)
            .into_iter()
            .flatten()
            .filter_map(|e| {
                let e = e.ok()?;
                let path = e.path();
                if !path.is_dir() {
                    return None;
                }
                let name = e.file_name().into_string().ok()?;
                let created = e
                    .metadata()
                    .ok()
                    .and_then(|m| m.created().ok())
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                Some(crate::BucketSummary { name, created })
            })
            .collect();
        summaries.sort_by(|a, b| a.name.cmp(&b.name));
        summaries
    }

    fn put_object(&self, bucket: &str, key: &str, input: PutObject) -> Result<()> {
        self.require_bucket(bucket)?;

        let etag = format!("\"{:x}\"", Md5::digest(&input.bytes));
        let size = input.bytes.len() as u64;

        fs::write(self.object_path(bucket, key), &input.bytes).expect("write object");

        let meta = MetaFile {
            etag,
            content_type: input.content_type,
            metadata: input.metadata,
            last_modified: now_secs(),
            size,
        };

        let meta_json = serde_json::to_vec(&meta).expect("serialize meta");
        fs::write(self.meta_path(bucket, key), &meta_json).expect("write meta");

        Ok(())
    }

    fn get_object(&self, bucket: &str, key: &str) -> Result<GetObject> {
        self.require_bucket(bucket)?;

        let bytes =
            fs::read(self.object_path(bucket, key)).map_err(|_| StoreError::ObjectNotFound {
                bucket: bucket.to_string(),
                key: key.to_string(),
            })?;

        let mf = self.read_meta(bucket, key)?;

        Ok(GetObject {
            bytes,
            meta: ObjectMeta {
                size: mf.size,
                etag: mf.etag,
                content_type: mf.content_type,
                last_modified: mf.last_modified,
                metadata: mf.metadata,
            },
        })
    }

    fn head_object(&self, bucket: &str, key: &str) -> Result<ObjectMeta> {
        self.require_bucket(bucket)?;
        let mf = self.read_meta(bucket, key)?;

        Ok(ObjectMeta {
            size: mf.size,
            etag: mf.etag,
            content_type: mf.content_type,
            last_modified: mf.last_modified,
            metadata: mf.metadata,
        })
    }

    fn delete_object(&self, bucket: &str, key: &str) -> Result<()> {
        self.require_bucket(bucket)?;

        let obj_path = self.object_path(bucket, key);
        if !obj_path.exists() {
            return Err(StoreError::ObjectNotFound {
                bucket: bucket.to_string(),
                key: key.to_string(),
            });
        }

        fs::remove_file(obj_path).ok();
        fs::remove_file(self.meta_path(bucket, key)).ok();
        Ok(())
    }

    fn list_objects(&self, bucket: &str, prefix: Option<&str>) -> Result<Vec<ObjectSummary>> {
        self.require_bucket(bucket)?;

        let objects_dir = self.objects_dir(bucket);
        let mut summaries: Vec<ObjectSummary> = fs::read_dir(&objects_dir)
            .into_iter()
            .flatten()
            .filter_map(|e| {
                let e = e.ok()?;
                let encoded = e.file_name().into_string().ok()?;
                let key = decode_key(&encoded);

                if let Some(pfx) = prefix
                    && !key.starts_with(pfx)
                {
                    return None;
                }

                let mf = self.read_meta(bucket, &key).ok()?;
                Some(ObjectSummary {
                    key,
                    size: mf.size,
                    etag: mf.etag,
                    last_modified: mf.last_modified,
                })
            })
            .collect();

        summaries.sort_by(|a, b| a.key.cmp(&b.key));
        Ok(summaries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn setup() -> (TempDir, FsStore) {
        let tmp = TempDir::new().unwrap();
        let store = FsStore::new(tmp.path());
        (tmp, store)
    }

    fn put(store: &FsStore, bucket: &str, key: &str, bytes: &[u8]) {
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
    fn bucket_lifecycle() {
        let (_tmp, store) = setup();
        assert!(!store.bucket_exists("b"));
        store.create_bucket("b").unwrap();
        assert!(store.bucket_exists("b"));
        store.delete_bucket("b").unwrap();
        assert!(!store.bucket_exists("b"));
    }

    #[test]
    fn list_buckets() {
        let (_tmp, store) = setup();
        store.create_bucket("bravo").unwrap();
        store.create_bucket("alpha").unwrap();
        let buckets = store.list_buckets();
        let names: Vec<&str> = buckets.iter().map(|b| b.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "bravo"]);
        assert!(buckets[0].created > 0);
    }

    #[test]
    fn object_crud() {
        let (_tmp, store) = setup();
        store.create_bucket("b").unwrap();
        put(&store, "b", "key", b"hello");

        let obj = store.get_object("b", "key").unwrap();
        assert_eq!(obj.bytes, b"hello");
        assert_eq!(obj.meta.size, 5);

        store.delete_object("b", "key").unwrap();
        assert!(store.get_object("b", "key").is_err());
    }

    #[test]
    fn head_object() {
        let (_tmp, store) = setup();
        store.create_bucket("b").unwrap();
        store
            .put_object(
                "b",
                "k",
                PutObject {
                    bytes: b"data".to_vec(),
                    content_type: "text/plain".to_string(),
                    metadata: HashMap::from([("color".to_string(), "red".to_string())]),
                },
            )
            .unwrap();

        let meta = store.head_object("b", "k").unwrap();
        assert_eq!(meta.content_type, "text/plain");
        assert_eq!(meta.metadata.get("color").unwrap(), "red");
    }

    #[test]
    fn list_with_prefix() {
        let (_tmp, store) = setup();
        store.create_bucket("b").unwrap();
        put(&store, "b", "a/1.txt", b"one");
        put(&store, "b", "a/2.txt", b"two");
        put(&store, "b", "b/1.txt", b"three");

        let listed = store.list_objects("b", Some("a/")).unwrap();
        let keys: Vec<&str> = listed.iter().map(|o| o.key.as_str()).collect();
        assert_eq!(keys, vec!["a/1.txt", "a/2.txt"]);
    }

    #[test]
    fn delete_nonempty_bucket_fails() {
        let (_tmp, store) = setup();
        store.create_bucket("b").unwrap();
        put(&store, "b", "file", b"data");
        assert!(store.delete_bucket("b").is_err());
    }
}
