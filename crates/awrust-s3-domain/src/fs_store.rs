use crate::{
    GetObject, ListObjectsPage, ListObjectsParams, ObjectMeta, ObjectSummary, PutObject, Result,
    Store, StoreError, apply_delimiter, composite_etag, decode_continuation_token,
};
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

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

#[derive(Serialize, Deserialize)]
struct UploadMeta {
    key: String,
    content_type: String,
    metadata: HashMap<String, String>,
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

    fn uploads_dir(&self, bucket: &str) -> PathBuf {
        self.bucket_path(bucket).join("uploads")
    }

    fn upload_dir(&self, bucket: &str, upload_id: &str) -> PathBuf {
        self.uploads_dir(bucket).join(upload_id)
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
        fs::create_dir_all(self.objects_dir(name)).ok();
        fs::create_dir_all(self.meta_dir(name)).ok();
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
                if !e.path().is_dir() {
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

    fn list_objects(&self, bucket: &str, params: &ListObjectsParams) -> Result<ListObjectsPage> {
        self.require_bucket(bucket)?;

        let after_key = params
            .continuation_token
            .as_deref()
            .and_then(decode_continuation_token);

        let objects_dir = self.objects_dir(bucket);
        let mut summaries: Vec<ObjectSummary> = fs::read_dir(&objects_dir)
            .into_iter()
            .flatten()
            .filter_map(|e| {
                let e = e.ok()?;
                let encoded = e.file_name().into_string().ok()?;
                let key = decode_key(&encoded);

                if let Some(pfx) = &params.prefix
                    && !key.starts_with(pfx.as_str())
                {
                    return None;
                }

                if let Some(last) = &after_key
                    && key.as_str() <= last.as_str()
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

        let prefix = params.prefix.as_deref().unwrap_or("");
        Ok(apply_delimiter(
            summaries,
            prefix,
            params.delimiter.as_deref(),
            params.max_keys,
        ))
    }

    fn initiate_multipart(
        &self,
        bucket: &str,
        key: &str,
        content_type: &str,
        metadata: HashMap<String, String>,
    ) -> Result<String> {
        self.require_bucket(bucket)?;

        let upload_id = Uuid::new_v4().to_string();
        let upload_dir = self.upload_dir(bucket, &upload_id);
        let parts_dir = upload_dir.join("parts");
        fs::create_dir_all(&parts_dir).expect("create upload dir");

        let meta = UploadMeta {
            key: key.to_string(),
            content_type: content_type.to_string(),
            metadata,
        };
        let meta_json = serde_json::to_vec(&meta).expect("serialize upload meta");
        fs::write(upload_dir.join("meta.json"), &meta_json).expect("write upload meta");

        Ok(upload_id)
    }

    fn upload_part(
        &self,
        bucket: &str,
        _key: &str,
        upload_id: &str,
        part_number: u32,
        bytes: Vec<u8>,
    ) -> Result<String> {
        let upload_dir = self.upload_dir(bucket, upload_id);
        if !upload_dir.is_dir() {
            return Err(StoreError::UploadNotFound(upload_id.to_string()));
        }

        let etag = format!("\"{:x}\"", Md5::digest(&bytes));
        let part_path = upload_dir.join("parts").join(format!("{part_number:05}"));
        fs::write(part_path, &bytes).expect("write part");

        Ok(etag)
    }

    fn complete_multipart(
        &self,
        bucket: &str,
        _key: &str,
        upload_id: &str,
        parts: &[(u32, String)],
    ) -> Result<String> {
        let upload_dir = self.upload_dir(bucket, upload_id);
        if !upload_dir.is_dir() {
            return Err(StoreError::UploadNotFound(upload_id.to_string()));
        }

        let upload_meta: UploadMeta =
            serde_json::from_slice(&fs::read(upload_dir.join("meta.json")).expect("read meta"))
                .expect("parse meta");

        let mut assembled = Vec::new();
        let mut part_digests = Vec::new();

        for (part_number, _etag) in parts {
            let part_path = upload_dir.join("parts").join(format!("{part_number:05}"));
            let part_bytes = fs::read(&part_path).map_err(|_| StoreError::InvalidPart {
                upload_id: upload_id.to_string(),
                part_number: *part_number,
            })?;
            part_digests.push(Md5::digest(&part_bytes).to_vec());
            assembled.extend_from_slice(&part_bytes);
        }

        let etag = composite_etag(&part_digests);
        let size = assembled.len() as u64;

        fs::write(self.object_path(bucket, &upload_meta.key), &assembled).expect("write object");

        let meta = MetaFile {
            etag: etag.clone(),
            content_type: upload_meta.content_type,
            metadata: upload_meta.metadata,
            last_modified: now_secs(),
            size,
        };
        let meta_json = serde_json::to_vec(&meta).expect("serialize meta");
        fs::write(self.meta_path(bucket, &upload_meta.key), &meta_json).expect("write meta");

        fs::remove_dir_all(&upload_dir).ok();

        Ok(etag)
    }

    fn abort_multipart(&self, bucket: &str, _key: &str, upload_id: &str) -> Result<()> {
        let upload_dir = self.upload_dir(bucket, upload_id);
        if !upload_dir.is_dir() {
            return Err(StoreError::UploadNotFound(upload_id.to_string()));
        }
        fs::remove_dir_all(&upload_dir).ok();
        Ok(())
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

        let page = store
            .list_objects(
                "b",
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
    fn delete_nonempty_bucket_fails() {
        let (_tmp, store) = setup();
        store.create_bucket("b").unwrap();
        put(&store, "b", "file", b"data");
        assert!(store.delete_bucket("b").is_err());
    }

    #[test]
    fn multipart_upload() {
        let (_tmp, store) = setup();
        store.create_bucket("b").unwrap();

        let upload_id = store
            .initiate_multipart("b", "big", "application/octet-stream", HashMap::new())
            .unwrap();
        let etag1 = store
            .upload_part("b", "big", &upload_id, 1, b"part1".to_vec())
            .unwrap();
        let etag2 = store
            .upload_part("b", "big", &upload_id, 2, b"part2".to_vec())
            .unwrap();
        store
            .complete_multipart("b", "big", &upload_id, &[(1, etag1), (2, etag2)])
            .unwrap();

        let obj = store.get_object("b", "big").unwrap();
        assert_eq!(obj.bytes, b"part1part2");
    }

    #[test]
    fn copy_via_get_put() {
        let (_tmp, store) = setup();
        store.create_bucket("src").unwrap();
        store.create_bucket("dst").unwrap();
        store
            .put_object(
                "src",
                "file.txt",
                PutObject {
                    bytes: b"payload".to_vec(),
                    content_type: "text/plain".to_string(),
                    metadata: HashMap::from([("tag".to_string(), "v1".to_string())]),
                },
            )
            .unwrap();

        let obj = store.get_object("src", "file.txt").unwrap();
        store
            .put_object(
                "dst",
                "copy.txt",
                PutObject {
                    bytes: obj.bytes,
                    content_type: obj.meta.content_type,
                    metadata: obj.meta.metadata,
                },
            )
            .unwrap();

        let copied = store.get_object("dst", "copy.txt").unwrap();
        assert_eq!(copied.bytes, b"payload");
        assert_eq!(copied.meta.content_type, "text/plain");
        assert_eq!(copied.meta.metadata.get("tag").unwrap(), "v1");
    }

    #[test]
    fn pagination() {
        let (_tmp, store) = setup();
        store.create_bucket("b").unwrap();
        for i in 0..5 {
            put(&store, "b", &format!("key{i:02}"), b"data");
        }

        let page1 = store
            .list_objects(
                "b",
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
                "b",
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
                "b",
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
}
