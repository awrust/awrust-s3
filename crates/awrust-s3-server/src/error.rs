use awrust_s3_domain::StoreError;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub struct S3Error(pub StoreError);

impl From<StoreError> for S3Error {
    fn from(err: StoreError) -> Self {
        Self(err)
    }
}

impl IntoResponse for S3Error {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self.0 {
            StoreError::BucketNotFound(b) => (
                StatusCode::NOT_FOUND,
                "NoSuchBucket",
                format!("The specified bucket does not exist: {b}"),
            ),
            StoreError::BucketNotEmpty(b) => (
                StatusCode::CONFLICT,
                "BucketNotEmpty",
                format!("The bucket you tried to delete is not empty: {b}"),
            ),
            StoreError::ObjectNotFound { bucket, key } => (
                StatusCode::NOT_FOUND,
                "NoSuchKey",
                format!("The specified key does not exist: {bucket}/{key}"),
            ),
            StoreError::UploadNotFound(id) => (
                StatusCode::NOT_FOUND,
                "NoSuchUpload",
                format!("The specified upload does not exist: {id}"),
            ),
            StoreError::InvalidPart {
                upload_id,
                part_number,
            } => (
                StatusCode::BAD_REQUEST,
                "InvalidPart",
                format!("Invalid part {part_number} for upload {upload_id}"),
            ),
            StoreError::InvalidChunkedEncoding => (
                StatusCode::BAD_REQUEST,
                "InvalidArgument",
                "Could not decode aws-chunked request body".to_string(),
            ),
        };

        let body = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
             <Error><Code>{code}</Code><Message>{message}</Message></Error>"
        );

        (status, [("content-type", "application/xml")], body).into_response()
    }
}
