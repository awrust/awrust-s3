mod bucket;
mod format;
mod object;

use crate::error::S3Error;
use serde::Deserialize;

pub use bucket::{
    create_bucket, delete_bucket, get_bucket, head_bucket, list_buckets, post_bucket,
};
pub use object::{
    delete_object_or_abort, get_object, head_object, post_object, put_object_or_part,
};

type S3Result<T> = Result<T, S3Error>;

#[derive(Deserialize, Default)]
pub struct BucketQueryParams {
    pub delete: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct ListParams {
    pub prefix: Option<String>,
    pub delimiter: Option<String>,
    #[serde(rename = "max-keys")]
    pub max_keys: Option<usize>,
    #[serde(rename = "continuation-token")]
    pub continuation_token: Option<String>,
    pub uploads: Option<String>,
    pub versions: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct ObjectQueryParams {
    #[serde(rename = "uploadId")]
    pub upload_id: Option<String>,
    #[serde(rename = "partNumber")]
    pub part_number: Option<u32>,
    pub uploads: Option<String>,
    pub tagging: Option<String>,
}
