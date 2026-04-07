use std::fmt;

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
