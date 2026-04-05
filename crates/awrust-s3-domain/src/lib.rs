mod fs_store;

pub use fs_store::FsStore;

use md5::{Digest, Md5};
use std::collections::HashMap;
use std::fmt;
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

pub type Result<T> = std::result::Result<T, StoreError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    BucketNotFound(String),
    BucketNotEmpty(String),
    ObjectNotFound { bucket: String, key: String },
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::BucketNotFound(bucket) => write!(f, "bucket not found: {bucket}"),
            StoreError::BucketNotEmpty(bucket) => write!(f, "bucket not empty: {bucket}"),
            StoreError::ObjectNotFound { bucket, key } => {
                write!(f, "object not found: {bucket}/{key}")
            }
        }
    }
}

impl std::error::Error for StoreError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectSummary {
    pub key: String,
    pub size: u64,
    pub etag: String,
    pub last_modified: u64,
}

#[derive(Debug, Clone)]
pub struct ObjectMeta {
    pub size: u64,
    pub etag: String,
    pub content_type: String,
    pub last_modified: u64,
    pub metadata: HashMap<String, String>,
}

pub struct PutObject {
    pub bytes: Vec<u8>,
    pub content_type: String,
    pub metadata: HashMap<String, String>,
}

pub struct GetObject {
    pub bytes: Vec<u8>,
    pub meta: ObjectMeta,
}

pub trait Store: Send + Sync {
    fn create_bucket(&self, name: &str) -> Result<()>;
    fn bucket_exists(&self, name: &str) -> bool;
    fn delete_bucket(&self, name: &str) -> Result<()>;
    fn list_buckets(&self) -> Vec<String>;

    fn put_object(&self, bucket: &str, key: &str, input: PutObject) -> Result<()>;
    fn get_object(&self, bucket: &str, key: &str) -> Result<GetObject>;
    fn head_object(&self, bucket: &str, key: &str) -> Result<ObjectMeta>;
    fn delete_object(&self, bucket: &str, key: &str) -> Result<()>;

    fn list_objects(&self, bucket: &str, prefix: Option<&str>) -> Result<Vec<ObjectSummary>>;
}

#[derive(Debug, Default)]
pub struct MemoryStore {
    buckets: RwLock<HashMap<String, BucketState>>,
}

#[derive(Debug, Default)]
struct BucketState {
    objects: HashMap<String, ObjectRecord>,
}

#[derive(Debug)]
struct ObjectRecord {
    bytes: Vec<u8>,
    etag: String,
    content_type: String,
    metadata: HashMap<String, String>,
    last_modified: u64,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time after epoch")
        .as_secs()
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Store for MemoryStore {
    fn create_bucket(&self, name: &str) -> Result<()> {
        let mut buckets = self.buckets.write().expect("lock poisoned");
        buckets.entry(name.to_string()).or_default();
        Ok(())
    }

    fn bucket_exists(&self, name: &str) -> bool {
        let buckets = self.buckets.read().expect("lock poisoned");
        buckets.contains_key(name)
    }

    fn delete_bucket(&self, name: &str) -> Result<()> {
        let mut buckets = self.buckets.write().expect("lock poisoned");
        let bucket = buckets
            .get(name)
            .ok_or_else(|| StoreError::BucketNotFound(name.to_string()))?;

        if !bucket.objects.is_empty() {
            return Err(StoreError::BucketNotEmpty(name.to_string()));
        }

        buckets.remove(name);
        Ok(())
    }

    fn list_buckets(&self) -> Vec<String> {
        let buckets = self.buckets.read().expect("lock poisoned");
        let mut names: Vec<String> = buckets.keys().cloned().collect();
        names.sort();
        names
    }

    fn put_object(&self, bucket: &str, key: &str, input: PutObject) -> Result<()> {
        let mut buckets = self.buckets.write().expect("lock poisoned");
        let bucket_state = buckets
            .get_mut(bucket)
            .ok_or_else(|| StoreError::BucketNotFound(bucket.to_string()))?;

        let etag = format!("\"{:x}\"", Md5::digest(&input.bytes));
        bucket_state.objects.insert(
            key.to_string(),
            ObjectRecord {
                bytes: input.bytes,
                etag,
                content_type: input.content_type,
                metadata: input.metadata,
                last_modified: now_secs(),
            },
        );
        Ok(())
    }

    fn get_object(&self, bucket: &str, key: &str) -> Result<GetObject> {
        let buckets = self.buckets.read().expect("lock poisoned");
        let bucket_state = buckets
            .get(bucket)
            .ok_or_else(|| StoreError::BucketNotFound(bucket.to_string()))?;

        let record = bucket_state
            .objects
            .get(key)
            .ok_or_else(|| StoreError::ObjectNotFound {
                bucket: bucket.to_string(),
                key: key.to_string(),
            })?;

        Ok(GetObject {
            bytes: record.bytes.clone(),
            meta: ObjectMeta {
                size: record.bytes.len() as u64,
                etag: record.etag.clone(),
                content_type: record.content_type.clone(),
                last_modified: record.last_modified,
                metadata: record.metadata.clone(),
            },
        })
    }

    fn head_object(&self, bucket: &str, key: &str) -> Result<ObjectMeta> {
        let buckets = self.buckets.read().expect("lock poisoned");
        let bucket_state = buckets
            .get(bucket)
            .ok_or_else(|| StoreError::BucketNotFound(bucket.to_string()))?;

        let record = bucket_state
            .objects
            .get(key)
            .ok_or_else(|| StoreError::ObjectNotFound {
                bucket: bucket.to_string(),
                key: key.to_string(),
            })?;

        Ok(ObjectMeta {
            size: record.bytes.len() as u64,
            etag: record.etag.clone(),
            content_type: record.content_type.clone(),
            last_modified: record.last_modified,
            metadata: record.metadata.clone(),
        })
    }

    fn delete_object(&self, bucket: &str, key: &str) -> Result<()> {
        let mut buckets = self.buckets.write().expect("lock poisoned");
        let bucket_state = buckets
            .get_mut(bucket)
            .ok_or_else(|| StoreError::BucketNotFound(bucket.to_string()))?;

        if bucket_state.objects.remove(key).is_none() {
            return Err(StoreError::ObjectNotFound {
                bucket: bucket.to_string(),
                key: key.to_string(),
            });
        }

        Ok(())
    }

    fn list_objects(&self, bucket: &str, prefix: Option<&str>) -> Result<Vec<ObjectSummary>> {
        let buckets = self.buckets.read().expect("lock poisoned");
        let bucket_state = buckets
            .get(bucket)
            .ok_or_else(|| StoreError::BucketNotFound(bucket.to_string()))?;

        let mut summaries: Vec<ObjectSummary> = bucket_state
            .objects
            .iter()
            .filter(|(key, _)| match prefix {
                Some(prefix) => key.starts_with(prefix),
                None => true,
            })
            .map(|(key, record)| ObjectSummary {
                key: key.clone(),
                size: record.bytes.len() as u64,
                etag: record.etag.clone(),
                last_modified: record.last_modified,
            })
            .collect();

        summaries.sort_by(|a, b| a.key.cmp(&b.key));
        Ok(summaries)
    }
}
