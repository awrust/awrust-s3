use awrust_s3_domain::Store;
use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use md5::{Digest, Md5};
use serde::Deserialize;
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};

use crate::error::S3Error;
use crate::xml::{ListBucketResult, ObjectEntry, XmlResponse};

type S3Result<T> = Result<T, S3Error>;

pub async fn create_bucket(
    State(store): State<Arc<dyn Store>>,
    Path(bucket): Path<String>,
) -> S3Result<StatusCode> {
    store.create_bucket(&bucket)?;
    Ok(StatusCode::OK)
}

pub async fn head_bucket(
    State(store): State<Arc<dyn Store>>,
    Path(bucket): Path<String>,
) -> S3Result<StatusCode> {
    if store.bucket_exists(&bucket) {
        Ok(StatusCode::OK)
    } else {
        Err(awrust_s3_domain::StoreError::BucketNotFound(bucket).into())
    }
}

pub async fn delete_bucket(
    State(store): State<Arc<dyn Store>>,
    Path(bucket): Path<String>,
) -> S3Result<StatusCode> {
    store.delete_bucket(&bucket)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize, Default)]
pub struct ListParams {
    pub prefix: Option<String>,
    #[serde(rename = "max-keys")]
    pub max_keys: Option<usize>,
}

pub async fn list_objects(
    State(store): State<Arc<dyn Store>>,
    Path(bucket): Path<String>,
    Query(params): Query<ListParams>,
) -> S3Result<Response> {
    let prefix = params.prefix.as_deref();
    let objects = store.list_objects(&bucket, prefix)?;
    let max_keys = params.max_keys.unwrap_or(1000);

    let truncated = objects.len() > max_keys;
    let objects: Vec<_> = objects.into_iter().take(max_keys).collect();

    let result = ListBucketResult {
        name: bucket,
        prefix: prefix.unwrap_or_default().to_string(),
        key_count: objects.len(),
        max_keys,
        is_truncated: truncated,
        contents: objects
            .into_iter()
            .map(|o| ObjectEntry {
                key: o.key,
                last_modified: format_iso8601(o.last_modified),
                size: o.size,
                etag: o.etag,
            })
            .collect(),
    };

    Ok(XmlResponse(result).into_response())
}

pub async fn put_object(
    State(store): State<Arc<dyn Store>>,
    Path((bucket, key)): Path<(String, String)>,
    body: Bytes,
) -> S3Result<Response> {
    let etag = format!("\"{:x}\"", Md5::digest(&body));
    store.put_object(&bucket, &key, body.to_vec())?;
    Ok((StatusCode::OK, [("etag", etag)]).into_response())
}

pub async fn get_object(
    State(store): State<Arc<dyn Store>>,
    Path((bucket, key)): Path<(String, String)>,
) -> S3Result<Response> {
    let bytes = store.get_object(&bucket, &key)?;
    let etag = format!("\"{:x}\"", Md5::digest(&bytes));
    let len = bytes.len().to_string();

    let mut headers = HeaderMap::new();
    headers.insert("etag", etag.parse().expect("valid header"));
    headers.insert("content-length", len.parse().expect("valid header"));

    Ok((StatusCode::OK, headers, bytes).into_response())
}

pub async fn head_object(
    State(store): State<Arc<dyn Store>>,
    Path((bucket, key)): Path<(String, String)>,
) -> S3Result<Response> {
    let bytes = store.get_object(&bucket, &key)?;
    let etag = format!("\"{:x}\"", Md5::digest(&bytes));
    let len = bytes.len().to_string();

    let mut headers = HeaderMap::new();
    headers.insert("etag", etag.parse().expect("valid header"));
    headers.insert("content-length", len.parse().expect("valid header"));

    Ok((StatusCode::OK, headers).into_response())
}

pub async fn delete_object(
    State(store): State<Arc<dyn Store>>,
    Path((bucket, key)): Path<(String, String)>,
) -> S3Result<StatusCode> {
    store.delete_object(&bucket, &key)?;
    Ok(StatusCode::NO_CONTENT)
}

fn format_iso8601(epoch_secs: u64) -> String {
    let dt = UNIX_EPOCH + Duration::from_secs(epoch_secs);
    let secs = dt
        .duration_since(UNIX_EPOCH)
        .expect("after epoch")
        .as_secs();

    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;

    let (year, month, day) = days_to_ymd(days);
    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}.000Z")
}

fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    let mut year = 1970;
    loop {
        let year_days = if is_leap(year) { 366 } else { 365 };
        if days < year_days {
            break;
        }
        days -= year_days;
        year += 1;
    }

    let leap = is_leap(year);
    let month_days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];

    let mut month = 0;
    for (i, &md) in month_days.iter().enumerate() {
        if days < md {
            month = i as u64 + 1;
            break;
        }
        days -= md;
    }

    (year, month, days + 1)
}

fn is_leap(year: u64) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}
