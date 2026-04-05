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
            StoreError::BucketNotFound(bucket) => {
                write!(f, "bucket not found: {bucket}")
            }
            StoreError::BucketNotEmpty(bucket) => {
                write!(f, "bucket not empty: {bucket}")
            }
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

pub trait Store: Send + Sync {
    fn create_bucket(&self, name: &str) -> Result<()>;
    fn bucket_exists(&self, name: &str) -> bool;
    fn delete_bucket(&self, name: &str) -> Result<()>;

    fn put_object(&self, bucket: &str, key: &str, bytes: Vec<u8>) -> Result<()>;
    fn get_object(&self, bucket: &str, key: &str) -> Result<Vec<u8>>;
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

    fn put_object(&self, bucket: &str, key: &str, bytes: Vec<u8>) -> Result<()> {
        let mut buckets = self.buckets.write().expect("lock poisoned");
        let bucket_state = buckets
            .get_mut(bucket)
            .ok_or_else(|| StoreError::BucketNotFound(bucket.to_string()))?;

        bucket_state.objects.insert(
            key.to_string(),
            ObjectRecord {
                bytes,
                last_modified: now_secs(),
            },
        );
        Ok(())
    }

    fn get_object(&self, bucket: &str, key: &str) -> Result<Vec<u8>> {
        let buckets = self.buckets.read().expect("lock poisoned");
        let bucket_state = buckets
            .get(bucket)
            .ok_or_else(|| StoreError::BucketNotFound(bucket.to_string()))?;

        bucket_state
            .objects
            .get(key)
            .map(|r| r.bytes.clone())
            .ok_or_else(|| StoreError::ObjectNotFound {
                bucket: bucket.to_string(),
                key: key.to_string(),
            })
    }

    fn delete_object(&self, bucket: &str, key: &str) -> Result<()> {
        let mut buckets = self.buckets.write().expect("lock poisoned");
        let bucket_state = buckets
            .get_mut(bucket)
            .ok_or_else(|| StoreError::BucketNotFound(bucket.to_string()))?;

        let removed = bucket_state.objects.remove(key);
        if removed.is_none() {
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
                etag: format!("\"{:x}\"", Md5::digest(&record.bytes)),
                last_modified: record.last_modified,
            })
            .collect();

        summaries.sort_by(|a, b| a.key.cmp(&b.key));
        Ok(summaries)
    }
}
