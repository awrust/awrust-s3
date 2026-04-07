use md5::{Digest, Md5};
use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

use crate::error::{Result, StoreError};
use crate::store::Store;
use crate::types::{
    BucketSummary, GetObject, ListObjectsPage, ListObjectsParams, ObjectMeta, ObjectSummary,
    PutObject, UploadSummary,
};
use crate::util::{apply_delimiter, composite_etag, decode_continuation_token, now_secs};

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
    tags: HashMap<String, String>,
}

#[derive(Debug)]
struct InFlightUpload {
    bucket: String,
    key: String,
    content_type: String,
    metadata: HashMap<String, String>,
    parts: HashMap<u32, Vec<u8>>,
    initiated: u64,
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
                tags: HashMap::new(),
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
                initiated: now_secs(),
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
                tags: HashMap::new(),
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

    fn list_multipart_uploads(
        &self,
        bucket: &str,
        prefix: Option<&str>,
    ) -> Result<Vec<UploadSummary>> {
        if !self.bucket_exists(bucket) {
            return Err(StoreError::BucketNotFound(bucket.to_string()));
        }

        let uploads = self.uploads.read().expect("lock poisoned");
        let mut summaries: Vec<UploadSummary> = uploads
            .iter()
            .filter(|(_, u)| u.bucket == bucket)
            .filter(|(_, u)| match prefix {
                Some(pfx) => u.key.starts_with(pfx),
                None => true,
            })
            .map(|(id, u)| UploadSummary {
                key: u.key.clone(),
                upload_id: id.clone(),
                initiated: u.initiated,
            })
            .collect();
        summaries.sort_by(|a, b| a.key.cmp(&b.key));
        Ok(summaries)
    }

    fn put_object_tagging(
        &self,
        bucket: &str,
        key: &str,
        tags: HashMap<String, String>,
    ) -> Result<()> {
        let mut buckets = self.buckets.write().expect("lock poisoned");
        let bucket_state = buckets
            .get_mut(bucket)
            .ok_or_else(|| StoreError::BucketNotFound(bucket.to_string()))?;
        let record =
            bucket_state
                .objects
                .get_mut(key)
                .ok_or_else(|| StoreError::ObjectNotFound {
                    bucket: bucket.to_string(),
                    key: key.to_string(),
                })?;
        record.tags = tags;
        Ok(())
    }

    fn get_object_tagging(&self, bucket: &str, key: &str) -> Result<HashMap<String, String>> {
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
        Ok(record.tags.clone())
    }

    fn delete_object_tagging(&self, bucket: &str, key: &str) -> Result<()> {
        let mut buckets = self.buckets.write().expect("lock poisoned");
        let bucket_state = buckets
            .get_mut(bucket)
            .ok_or_else(|| StoreError::BucketNotFound(bucket.to_string()))?;
        let record =
            bucket_state
                .objects
                .get_mut(key)
                .ok_or_else(|| StoreError::ObjectNotFound {
                    bucket: bucket.to_string(),
                    key: key.to_string(),
                })?;
        record.tags.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_multipart_uploads_empty() {
        let store = MemoryStore::new();
        store.create_bucket("b").unwrap();
        let uploads = store.list_multipart_uploads("b", None).unwrap();
        assert!(uploads.is_empty());
    }

    #[test]
    fn list_multipart_uploads_returns_active() {
        let store = MemoryStore::new();
        store.create_bucket("b").unwrap();
        let id1 = store
            .initiate_multipart("b", "photos/cat.jpg", "image/jpeg", HashMap::new())
            .unwrap();
        let id2 = store
            .initiate_multipart("b", "docs/readme.md", "text/plain", HashMap::new())
            .unwrap();

        let uploads = store.list_multipart_uploads("b", None).unwrap();
        assert_eq!(uploads.len(), 2);
        assert_eq!(uploads[0].key, "docs/readme.md");
        assert_eq!(uploads[0].upload_id, id2);
        assert!(uploads[0].initiated > 0);
        assert_eq!(uploads[1].key, "photos/cat.jpg");
        assert_eq!(uploads[1].upload_id, id1);
    }

    #[test]
    fn list_multipart_uploads_filters_by_prefix() {
        let store = MemoryStore::new();
        store.create_bucket("b").unwrap();
        store
            .initiate_multipart("b", "photos/cat.jpg", "image/jpeg", HashMap::new())
            .unwrap();
        store
            .initiate_multipart("b", "docs/readme.md", "text/plain", HashMap::new())
            .unwrap();

        let uploads = store.list_multipart_uploads("b", Some("photos/")).unwrap();
        assert_eq!(uploads.len(), 1);
        assert_eq!(uploads[0].key, "photos/cat.jpg");
    }

    #[test]
    fn list_multipart_uploads_excludes_completed() {
        let store = MemoryStore::new();
        store.create_bucket("b").unwrap();
        let id = store
            .initiate_multipart("b", "key", "application/octet-stream", HashMap::new())
            .unwrap();
        let etag = store
            .upload_part("b", "key", &id, 1, b"data".to_vec())
            .unwrap();
        store
            .complete_multipart("b", "key", &id, &[(1, etag)])
            .unwrap();

        let uploads = store.list_multipart_uploads("b", None).unwrap();
        assert!(uploads.is_empty());
    }

    #[test]
    fn list_multipart_uploads_bucket_not_found() {
        let store = MemoryStore::new();
        let result = store.list_multipart_uploads("nope", None);
        assert!(matches!(result, Err(StoreError::BucketNotFound(_))));
    }

    #[test]
    fn list_multipart_uploads_scoped_to_bucket() {
        let store = MemoryStore::new();
        store.create_bucket("a").unwrap();
        store.create_bucket("b").unwrap();
        store
            .initiate_multipart("a", "key-a", "text/plain", HashMap::new())
            .unwrap();
        store
            .initiate_multipart("b", "key-b", "text/plain", HashMap::new())
            .unwrap();

        let uploads = store.list_multipart_uploads("a", None).unwrap();
        assert_eq!(uploads.len(), 1);
        assert_eq!(uploads[0].key, "key-a");
    }
}
