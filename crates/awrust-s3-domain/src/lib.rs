mod fs_store;

pub use fs_store::FsStore;

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use md5::{Digest, Md5};
use std::collections::HashMap;
use std::fmt;
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub type Result<T> = std::result::Result<T, StoreError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    BucketNotFound(String),
    BucketNotEmpty(String),
    ObjectNotFound { bucket: String, key: String },
    UploadNotFound(String),
    InvalidPart { upload_id: String, part_number: u32 },
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::BucketNotFound(bucket) => write!(f, "bucket not found: {bucket}"),
            StoreError::BucketNotEmpty(bucket) => write!(f, "bucket not empty: {bucket}"),
            StoreError::ObjectNotFound { bucket, key } => {
                write!(f, "object not found: {bucket}/{key}")
            }
            StoreError::UploadNotFound(id) => write!(f, "upload not found: {id}"),
            StoreError::InvalidPart {
                upload_id,
                part_number,
            } => write!(f, "invalid part {part_number} for upload {upload_id}"),
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

#[derive(Debug, Clone)]
pub struct BucketSummary {
    pub name: String,
    pub created: u64,
}

pub struct ListObjectsParams {
    pub prefix: Option<String>,
    pub continuation_token: Option<String>,
    pub max_keys: usize,
}

pub struct ListObjectsPage {
    pub objects: Vec<ObjectSummary>,
    pub is_truncated: bool,
    pub next_continuation_token: Option<String>,
}

pub trait Store: Send + Sync {
    fn create_bucket(&self, name: &str) -> Result<()>;
    fn bucket_exists(&self, name: &str) -> bool;
    fn delete_bucket(&self, name: &str) -> Result<()>;
    fn list_buckets(&self) -> Vec<BucketSummary>;

    fn put_object(&self, bucket: &str, key: &str, input: PutObject) -> Result<()>;
    fn get_object(&self, bucket: &str, key: &str) -> Result<GetObject>;
    fn head_object(&self, bucket: &str, key: &str) -> Result<ObjectMeta>;
    fn delete_object(&self, bucket: &str, key: &str) -> Result<()>;
    fn list_objects(&self, bucket: &str, params: &ListObjectsParams) -> Result<ListObjectsPage>;

    fn initiate_multipart(
        &self,
        bucket: &str,
        key: &str,
        content_type: &str,
        metadata: HashMap<String, String>,
    ) -> Result<String>;
    fn upload_part(
        &self,
        bucket: &str,
        key: &str,
        upload_id: &str,
        part_number: u32,
        bytes: Vec<u8>,
    ) -> Result<String>;
    fn complete_multipart(
        &self,
        bucket: &str,
        key: &str,
        upload_id: &str,
        parts: &[(u32, String)],
    ) -> Result<String>;
    fn abort_multipart(&self, bucket: &str, key: &str, upload_id: &str) -> Result<()>;
}

#[derive(Debug, Default)]
pub struct MemoryStore {
    buckets: RwLock<HashMap<String, BucketState>>,
    uploads: RwLock<HashMap<String, InFlightUpload>>,
}

#[derive(Debug)]
struct BucketState {
    created: u64,
    objects: HashMap<String, ObjectRecord>,
}

impl Default for BucketState {
    fn default() -> Self {
        Self {
            created: now_secs(),
            objects: HashMap::new(),
        }
    }
}

#[derive(Debug)]
struct ObjectRecord {
    bytes: Vec<u8>,
    etag: String,
    content_type: String,
    metadata: HashMap<String, String>,
    last_modified: u64,
}

#[derive(Debug)]
struct InFlightUpload {
    bucket: String,
    key: String,
    content_type: String,
    metadata: HashMap<String, String>,
    parts: HashMap<u32, Vec<u8>>,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time after epoch")
        .as_secs()
}

fn composite_etag(part_digests: &[Vec<u8>]) -> String {
    let mut hasher = Md5::new();
    for digest in part_digests {
        hasher.update(digest);
    }
    format!("\"{:x}-{}\"", hasher.finalize(), part_digests.len())
}

fn encode_continuation_token(key: &str) -> String {
    BASE64.encode(key.as_bytes())
}

fn decode_continuation_token(token: &str) -> Option<String> {
    BASE64
        .decode(token)
        .ok()
        .and_then(|b| String::from_utf8(b).ok())
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

    fn list_buckets(&self) -> Vec<BucketSummary> {
        let buckets = self.buckets.read().expect("lock poisoned");
        let mut summaries: Vec<BucketSummary> = buckets
            .iter()
            .map(|(name, state)| BucketSummary {
                name: name.clone(),
                created: state.created,
            })
            .collect();
        summaries.sort_by(|a, b| a.name.cmp(&b.name));
        summaries
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

    fn list_objects(&self, bucket: &str, params: &ListObjectsParams) -> Result<ListObjectsPage> {
        let buckets = self.buckets.read().expect("lock poisoned");
        let bucket_state = buckets
            .get(bucket)
            .ok_or_else(|| StoreError::BucketNotFound(bucket.to_string()))?;

        let after_key = params
            .continuation_token
            .as_deref()
            .and_then(decode_continuation_token);

        let mut summaries: Vec<ObjectSummary> = bucket_state
            .objects
            .iter()
            .filter(|(key, _)| match &params.prefix {
                Some(prefix) => key.starts_with(prefix.as_str()),
                None => true,
            })
            .filter(|(key, _)| match &after_key {
                Some(last) => key.as_str() > last.as_str(),
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

        let is_truncated = summaries.len() > params.max_keys;
        summaries.truncate(params.max_keys);

        let next_continuation_token = if is_truncated {
            summaries.last().map(|s| encode_continuation_token(&s.key))
        } else {
            None
        };

        Ok(ListObjectsPage {
            objects: summaries,
            is_truncated,
            next_continuation_token,
        })
    }

    fn initiate_multipart(
        &self,
        bucket: &str,
        key: &str,
        content_type: &str,
        metadata: HashMap<String, String>,
    ) -> Result<String> {
        if !self.bucket_exists(bucket) {
            return Err(StoreError::BucketNotFound(bucket.to_string()));
        }

        let upload_id = Uuid::new_v4().to_string();
        let mut uploads = self.uploads.write().expect("lock poisoned");
        uploads.insert(
            upload_id.clone(),
            InFlightUpload {
                bucket: bucket.to_string(),
                key: key.to_string(),
                content_type: content_type.to_string(),
                metadata,
                parts: HashMap::new(),
            },
        );
        Ok(upload_id)
    }

    fn upload_part(
        &self,
        _bucket: &str,
        _key: &str,
        upload_id: &str,
        part_number: u32,
        bytes: Vec<u8>,
    ) -> Result<String> {
        let mut uploads = self.uploads.write().expect("lock poisoned");
        let upload = uploads
            .get_mut(upload_id)
            .ok_or_else(|| StoreError::UploadNotFound(upload_id.to_string()))?;

        let etag = format!("\"{:x}\"", Md5::digest(&bytes));
        upload.parts.insert(part_number, bytes);
        Ok(etag)
    }

    fn complete_multipart(
        &self,
        _bucket: &str,
        _key: &str,
        upload_id: &str,
        parts: &[(u32, String)],
    ) -> Result<String> {
        let mut uploads = self.uploads.write().expect("lock poisoned");
        let upload = uploads
            .remove(upload_id)
            .ok_or_else(|| StoreError::UploadNotFound(upload_id.to_string()))?;

        let mut assembled = Vec::new();
        let mut part_digests = Vec::new();

        for (part_number, _etag) in parts {
            let part_bytes =
                upload
                    .parts
                    .get(part_number)
                    .ok_or_else(|| StoreError::InvalidPart {
                        upload_id: upload_id.to_string(),
                        part_number: *part_number,
                    })?;
            part_digests.push(Md5::digest(part_bytes).to_vec());
            assembled.extend_from_slice(part_bytes);
        }

        let etag = composite_etag(&part_digests);

        let mut buckets = self.buckets.write().expect("lock poisoned");
        let bucket_state = buckets
            .get_mut(&upload.bucket)
            .ok_or_else(|| StoreError::BucketNotFound(upload.bucket.clone()))?;

        bucket_state.objects.insert(
            upload.key,
            ObjectRecord {
                bytes: assembled,
                etag: etag.clone(),
                content_type: upload.content_type,
                metadata: upload.metadata,
                last_modified: now_secs(),
            },
        );

        Ok(etag)
    }

    fn abort_multipart(&self, _bucket: &str, _key: &str, upload_id: &str) -> Result<()> {
        let mut uploads = self.uploads.write().expect("lock poisoned");
        uploads
            .remove(upload_id)
            .ok_or_else(|| StoreError::UploadNotFound(upload_id.to_string()))?;
        Ok(())
    }
}
