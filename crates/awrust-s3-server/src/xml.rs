use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use quick_xml::se::to_string;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[serde(rename = "ListBucketResult")]
pub struct ListBucketResult {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Prefix")]
    pub prefix: String,
    #[serde(rename = "KeyCount")]
    pub key_count: usize,
    #[serde(rename = "MaxKeys")]
    pub max_keys: usize,
    #[serde(rename = "IsTruncated")]
    pub is_truncated: bool,
    #[serde(rename = "ContinuationToken", skip_serializing_if = "Option::is_none")]
    pub continuation_token: Option<String>,
    #[serde(
        rename = "NextContinuationToken",
        skip_serializing_if = "Option::is_none"
    )]
    pub next_continuation_token: Option<String>,
    #[serde(rename = "Contents", default)]
    pub contents: Vec<ObjectEntry>,
}

#[derive(Serialize)]
pub struct ObjectEntry {
    #[serde(rename = "Key")]
    pub key: String,
    #[serde(rename = "LastModified")]
    pub last_modified: String,
    #[serde(rename = "Size")]
    pub size: u64,
    #[serde(rename = "ETag")]
    pub etag: String,
}

#[derive(Serialize)]
#[serde(rename = "ListAllMyBucketsResult")]
pub struct ListAllMyBucketsResult {
    #[serde(rename = "Buckets")]
    pub buckets: BucketList,
}

#[derive(Serialize)]
pub struct BucketList {
    #[serde(rename = "Bucket")]
    pub entries: Vec<BucketEntry>,
}

#[derive(Serialize)]
pub struct BucketEntry {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "CreationDate")]
    pub creation_date: String,
}

#[derive(Serialize)]
#[serde(rename = "InitiateMultipartUploadResult")]
pub struct InitiateMultipartUploadResult {
    #[serde(rename = "Bucket")]
    pub bucket: String,
    #[serde(rename = "Key")]
    pub key: String,
    #[serde(rename = "UploadId")]
    pub upload_id: String,
}

#[derive(Serialize)]
#[serde(rename = "CompleteMultipartUploadResult")]
pub struct CompleteMultipartUploadResult {
    #[serde(rename = "Bucket")]
    pub bucket: String,
    #[serde(rename = "Key")]
    pub key: String,
    #[serde(rename = "ETag")]
    pub etag: String,
}

#[derive(Deserialize)]
#[serde(rename = "CompleteMultipartUpload")]
pub struct CompleteMultipartUploadRequest {
    #[serde(rename = "Part")]
    pub parts: Vec<CompletedPart>,
}

#[derive(Deserialize)]
pub struct CompletedPart {
    #[serde(rename = "PartNumber")]
    pub part_number: u32,
    #[serde(rename = "ETag")]
    pub etag: String,
}

#[derive(Deserialize)]
#[serde(rename = "Delete")]
pub struct DeleteRequest {
    #[serde(rename = "Quiet", default)]
    pub quiet: bool,
    #[serde(rename = "Object")]
    pub objects: Vec<DeleteRequestObject>,
}

#[derive(Deserialize)]
pub struct DeleteRequestObject {
    #[serde(rename = "Key")]
    pub key: String,
}

#[derive(Serialize)]
#[serde(rename = "DeleteResult")]
pub struct DeleteResult {
    #[serde(rename = "Deleted", default, skip_serializing_if = "Vec::is_empty")]
    pub deleted: Vec<DeletedEntry>,
    #[serde(rename = "Error", default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<DeleteErrorEntry>,
}

#[derive(Serialize)]
pub struct DeletedEntry {
    #[serde(rename = "Key")]
    pub key: String,
}

#[derive(Serialize)]
pub struct DeleteErrorEntry {
    #[serde(rename = "Key")]
    pub key: String,
    #[serde(rename = "Code")]
    pub code: String,
    #[serde(rename = "Message")]
    pub message: String,
}

pub struct XmlResponse<T: Serialize>(pub T);

impl<T: Serialize> IntoResponse for XmlResponse<T> {
    fn into_response(self) -> Response {
        match to_string(&self.0) {
            Ok(body) => {
                let xml = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>{body}");
                (StatusCode::OK, [("content-type", "application/xml")], xml).into_response()
            }
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
