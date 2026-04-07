use awrust_s3_domain::{ListObjectsParams, ObjectMeta, PutObject, Store};
use axum::body::Bytes;
use axum::extract::{Path, Query, RawQuery, State};
use axum::http::{HeaderMap, HeaderName, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};

use crate::error::S3Error;
use crate::xml::{
    BucketEntry, BucketList, CommonPrefix, CompleteMultipartUploadResult, CopyObjectResult,
    DeleteErrorEntry, DeleteResult, DeletedEntry, InitiateMultipartUploadResult,
    ListAllMyBucketsResult, ListBucketResult, ListMultipartUploadsResult, LocationConstraint,
    ObjectEntry, UploadEntry, XmlResponse,
};

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
pub struct BucketQueryParams {
    pub delete: Option<String>,
}

pub async fn post_bucket(
    State(store): State<Arc<dyn Store>>,
    Path(bucket): Path<String>,
    Query(params): Query<BucketQueryParams>,
    body: Bytes,
) -> S3Result<Response> {
    if params.delete.is_none() {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    }

    if !store.bucket_exists(&bucket) {
        return Err(awrust_s3_domain::StoreError::BucketNotFound(bucket).into());
    }

    let request: crate::xml::DeleteRequest =
        quick_xml::de::from_str(&String::from_utf8_lossy(&body))
            .map_err(|_| awrust_s3_domain::StoreError::BucketNotFound(bucket.clone()))?;

    let mut deleted = Vec::new();
    let mut errors = Vec::new();

    for obj in &request.objects {
        match store.delete_object(&bucket, &obj.key) {
            Ok(()) | Err(awrust_s3_domain::StoreError::ObjectNotFound { .. }) => {
                if !request.quiet {
                    deleted.push(DeletedEntry {
                        key: obj.key.clone(),
                    });
                }
            }
            Err(e) => {
                errors.push(DeleteErrorEntry {
                    key: obj.key.clone(),
                    code: "InternalError".to_string(),
                    message: e.to_string(),
                });
            }
        }
    }

    Ok(XmlResponse(DeleteResult { deleted, errors }).into_response())
}

pub async fn list_buckets(State(store): State<Arc<dyn Store>>) -> Response {
    let summaries = store.list_buckets();
    let result = ListAllMyBucketsResult {
        buckets: BucketList {
            entries: summaries
                .into_iter()
                .map(|b| BucketEntry {
                    name: b.name,
                    creation_date: format_iso8601(b.created),
                })
                .collect(),
        },
    };
    XmlResponse(result).into_response()
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
}

pub async fn get_bucket(
    state: State<Arc<dyn Store>>,
    path: Path<String>,
    RawQuery(query): RawQuery,
    params: Query<ListParams>,
) -> S3Result<Response> {
    if query.as_deref() == Some("location") {
        return get_bucket_location(state, path).await;
    }
    list_objects(state, path, params).await
}

async fn get_bucket_location(
    State(store): State<Arc<dyn Store>>,
    Path(bucket): Path<String>,
) -> S3Result<Response> {
    if !store.bucket_exists(&bucket) {
        return Err(awrust_s3_domain::StoreError::BucketNotFound(bucket).into());
    }
    Ok(XmlResponse(LocationConstraint {
        region: "us-east-1".to_string(),
    })
    .into_response())
}

pub async fn list_objects(
    State(store): State<Arc<dyn Store>>,
    Path(bucket): Path<String>,
    Query(params): Query<ListParams>,
) -> S3Result<Response> {
    if params.uploads.is_some() {
        let summaries = store.list_multipart_uploads(&bucket, params.prefix.as_deref())?;
        let result = ListMultipartUploadsResult {
            bucket,
            prefix: params.prefix.unwrap_or_default(),
            uploads: summaries
                .into_iter()
                .map(|u| UploadEntry {
                    key: u.key,
                    upload_id: u.upload_id,
                    initiated: format_iso8601(u.initiated),
                })
                .collect(),
        };
        return Ok(XmlResponse(result).into_response());
    }

    let max_keys = params.max_keys.unwrap_or(1000);
    let page = store.list_objects(
        &bucket,
        &ListObjectsParams {
            prefix: params.prefix.clone(),
            delimiter: params.delimiter.clone(),
            continuation_token: params.continuation_token.clone(),
            max_keys,
        },
    )?;

    let result = ListBucketResult {
        name: bucket,
        prefix: params.prefix.unwrap_or_default(),
        delimiter: params.delimiter,
        key_count: page.objects.len() + page.common_prefixes.len(),
        max_keys,
        is_truncated: page.is_truncated,
        continuation_token: params.continuation_token,
        next_continuation_token: page.next_continuation_token,
        contents: page
            .objects
            .into_iter()
            .map(|o| ObjectEntry {
                key: o.key,
                last_modified: format_iso8601(o.last_modified),
                size: o.size,
                etag: o.etag,
            })
            .collect(),
        common_prefixes: page
            .common_prefixes
            .into_iter()
            .map(|p| CommonPrefix { prefix: p })
            .collect(),
    };

    Ok(XmlResponse(result).into_response())
}

#[derive(Deserialize, Default)]
pub struct ObjectQueryParams {
    #[serde(rename = "uploadId")]
    pub upload_id: Option<String>,
    #[serde(rename = "partNumber")]
    pub part_number: Option<u32>,
    pub uploads: Option<String>,
}

pub async fn put_object_or_part(
    State(store): State<Arc<dyn Store>>,
    Path((bucket, key)): Path<(String, String)>,
    Query(params): Query<ObjectQueryParams>,
    headers: HeaderMap,
    body: Bytes,
) -> S3Result<Response> {
    if let Some(copy_source) = headers
        .get("x-amz-copy-source")
        .and_then(|v| v.to_str().ok())
    {
        return copy_object(&store, &bucket, &key, copy_source);
    }

    if let (Some(upload_id), Some(part_number)) = (&params.upload_id, params.part_number) {
        let etag = store.upload_part(&bucket, &key, upload_id, part_number, body.to_vec())?;
        return Ok((StatusCode::OK, [("etag", etag)]).into_response());
    }

    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();

    let metadata = extract_amz_meta(&headers);

    store.put_object(
        &bucket,
        &key,
        PutObject {
            bytes: body.to_vec(),
            content_type,
            metadata,
        },
    )?;

    let meta = store.head_object(&bucket, &key)?;
    Ok((StatusCode::OK, [("etag", meta.etag)]).into_response())
}

pub async fn post_object(
    State(store): State<Arc<dyn Store>>,
    Path((bucket, key)): Path<(String, String)>,
    Query(params): Query<ObjectQueryParams>,
    headers: HeaderMap,
    body: Bytes,
) -> S3Result<Response> {
    if params.uploads.is_some() {
        let content_type = headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();
        let metadata = extract_amz_meta(&headers);

        let upload_id = store.initiate_multipart(&bucket, &key, &content_type, metadata)?;

        let result = InitiateMultipartUploadResult {
            bucket,
            key,
            upload_id,
        };
        return Ok(XmlResponse(result).into_response());
    }

    if let Some(upload_id) = &params.upload_id {
        let request: crate::xml::CompleteMultipartUploadRequest =
            quick_xml::de::from_str(&String::from_utf8_lossy(&body))
                .map_err(|_| awrust_s3_domain::StoreError::UploadNotFound(upload_id.clone()))?;

        let parts: Vec<(u32, String)> = request
            .parts
            .into_iter()
            .map(|p| (p.part_number, p.etag))
            .collect();

        let etag = store.complete_multipart(&bucket, &key, upload_id, &parts)?;

        let result = CompleteMultipartUploadResult { bucket, key, etag };
        return Ok(XmlResponse(result).into_response());
    }

    Ok(StatusCode::BAD_REQUEST.into_response())
}

pub async fn get_object(
    State(store): State<Arc<dyn Store>>,
    Path((bucket, key)): Path<(String, String)>,
    headers: HeaderMap,
) -> S3Result<Response> {
    let obj = store.get_object(&bucket, &key)?;
    let total_size = obj.bytes.len();

    if let Some(range_header) = headers.get("range").and_then(|v| v.to_str().ok()) {
        if let Some((start, end)) = parse_range(range_header, total_size) {
            let slice = obj.bytes[start..=end].to_vec();
            let mut resp_headers = meta_to_headers(&obj.meta);
            resp_headers.insert(
                "content-range",
                format!("bytes {start}-{end}/{total_size}")
                    .parse()
                    .expect("valid header"),
            );
            resp_headers.insert(
                "content-length",
                slice.len().to_string().parse().expect("valid header"),
            );
            resp_headers.insert("accept-ranges", "bytes".parse().expect("valid header"));
            return Ok((StatusCode::PARTIAL_CONTENT, resp_headers, slice).into_response());
        }

        let mut resp_headers = HeaderMap::new();
        resp_headers.insert(
            "content-range",
            format!("bytes */{total_size}")
                .parse()
                .expect("valid header"),
        );
        return Ok((StatusCode::RANGE_NOT_SATISFIABLE, resp_headers).into_response());
    }

    let mut resp_headers = meta_to_headers(&obj.meta);
    resp_headers.insert("accept-ranges", "bytes".parse().expect("valid header"));
    Ok((StatusCode::OK, resp_headers, obj.bytes).into_response())
}

pub async fn head_object(
    State(store): State<Arc<dyn Store>>,
    Path((bucket, key)): Path<(String, String)>,
) -> S3Result<Response> {
    let meta = store.head_object(&bucket, &key)?;
    let mut headers = meta_to_headers(&meta);
    headers.insert("accept-ranges", "bytes".parse().expect("valid header"));
    Ok((StatusCode::OK, headers).into_response())
}

pub async fn delete_object_or_abort(
    State(store): State<Arc<dyn Store>>,
    Path((bucket, key)): Path<(String, String)>,
    Query(params): Query<ObjectQueryParams>,
) -> S3Result<StatusCode> {
    if let Some(upload_id) = &params.upload_id {
        store.abort_multipart(&bucket, &key, upload_id)?;
        return Ok(StatusCode::NO_CONTENT);
    }
    store.delete_object(&bucket, &key)?;
    Ok(StatusCode::NO_CONTENT)
}

fn copy_object(
    store: &Arc<dyn Store>,
    dst_bucket: &str,
    dst_key: &str,
    copy_source: &str,
) -> S3Result<Response> {
    let source = copy_source.strip_prefix('/').unwrap_or(copy_source);
    let (src_bucket, src_key) =
        source
            .split_once('/')
            .ok_or_else(|| awrust_s3_domain::StoreError::ObjectNotFound {
                bucket: source.to_string(),
                key: String::new(),
            })?;

    let obj = store.get_object(src_bucket, src_key)?;
    store.put_object(
        dst_bucket,
        dst_key,
        PutObject {
            bytes: obj.bytes,
            content_type: obj.meta.content_type,
            metadata: obj.meta.metadata,
        },
    )?;

    let meta = store.head_object(dst_bucket, dst_key)?;
    Ok(XmlResponse(CopyObjectResult {
        etag: meta.etag,
        last_modified: format_iso8601(meta.last_modified),
    })
    .into_response())
}

fn parse_range(header: &str, total: usize) -> Option<(usize, usize)> {
    let range_str = header.strip_prefix("bytes=")?;

    if let Some(suffix) = range_str.strip_prefix('-') {
        let n: usize = suffix.parse().ok()?;
        if n == 0 || n > total {
            return None;
        }
        return Some((total - n, total - 1));
    }

    let (start_str, end_str) = range_str.split_once('-')?;
    let start: usize = start_str.parse().ok()?;

    if end_str.is_empty() {
        if start >= total {
            return None;
        }
        return Some((start, total - 1));
    }

    let end: usize = end_str.parse().ok()?;
    if start > end || start >= total {
        return None;
    }
    Some((start, end.min(total - 1)))
}

fn extract_amz_meta(headers: &HeaderMap) -> HashMap<String, String> {
    headers
        .iter()
        .filter_map(|(name, value)| {
            let name_str = name.as_str();
            if let Some(key) = name_str.strip_prefix("x-amz-meta-") {
                value
                    .to_str()
                    .ok()
                    .map(|v| (key.to_string(), v.to_string()))
            } else {
                None
            }
        })
        .collect()
}

fn meta_to_headers(meta: &ObjectMeta) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert("etag", meta.etag.parse().expect("valid header"));
    headers.insert(
        "content-length",
        meta.size.to_string().parse().expect("valid header"),
    );
    headers.insert(
        "content-type",
        meta.content_type.parse().expect("valid header"),
    );
    headers.insert(
        "last-modified",
        format_iso8601(meta.last_modified)
            .parse()
            .expect("valid header"),
    );

    for (key, value) in &meta.metadata {
        let header_name = format!("x-amz-meta-{key}");
        if let (Ok(name), Ok(val)) = (header_name.parse::<HeaderName>(), value.parse()) {
            headers.insert(name, val);
        }
    }

    headers
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
