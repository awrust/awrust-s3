use crate::error::Result;
use crate::types::{
    BucketSummary, GetObject, ListObjectsPage, ListObjectsParams, ObjectMeta, PutObject,
    UploadSummary,
};
use std::collections::HashMap;

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
    fn list_multipart_uploads(
        &self,
        bucket: &str,
        prefix: Option<&str>,
    ) -> Result<Vec<UploadSummary>>;

    fn put_object_tagging(
        &self,
        bucket: &str,
        key: &str,
        tags: HashMap<String, String>,
    ) -> Result<()>;
    fn get_object_tagging(&self, bucket: &str, key: &str) -> Result<HashMap<String, String>>;
    fn delete_object_tagging(&self, bucket: &str, key: &str) -> Result<()>;
}
