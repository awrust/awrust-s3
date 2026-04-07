use std::collections::HashMap;

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
    pub delimiter: Option<String>,
    pub continuation_token: Option<String>,
    pub max_keys: usize,
}

pub struct ListObjectsPage {
    pub objects: Vec<ObjectSummary>,
    pub common_prefixes: Vec<String>,
    pub is_truncated: bool,
    pub next_continuation_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadSummary {
    pub key: String,
    pub upload_id: String,
    pub initiated: u64,
}
