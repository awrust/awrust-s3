use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use quick_xml::se::to_string;
use serde::Serialize;

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
