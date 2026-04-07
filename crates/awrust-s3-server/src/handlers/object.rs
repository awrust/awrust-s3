use awrust_s3_domain::{PutObject, Store};
use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use std::sync::Arc;

use super::format::{extract_amz_meta, format_iso8601, meta_to_headers, parse_range};
use super::{ObjectQueryParams, S3Result};
use crate::xml::{
    CompleteMultipartUploadResult, CopyObjectResult, InitiateMultipartUploadResult, Tagging,
    XmlResponse,
};

pub async fn put_object_or_part(
    State(store): State<Arc<dyn Store>>,
    Path((bucket, key)): Path<(String, String)>,
    Query(params): Query<ObjectQueryParams>,
    headers: HeaderMap,
    body: Bytes,
) -> S3Result<Response> {
    if params.tagging.is_some() {
        let tagging: Tagging =
            quick_xml::de::from_str(&String::from_utf8_lossy(&body)).map_err(|_| {
                awrust_s3_domain::StoreError::ObjectNotFound {
                    bucket: bucket.clone(),
                    key: key.clone(),
                }
            })?;
        store.put_object_tagging(&bucket, &key, tagging.into_map())?;
        return Ok(StatusCode::OK.into_response());
    }

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
    Query(params): Query<ObjectQueryParams>,
    headers: HeaderMap,
) -> S3Result<Response> {
    if params.tagging.is_some() {
        let tags = store.get_object_tagging(&bucket, &key)?;
        return Ok(XmlResponse(Tagging::from_map(tags)).into_response());
    }

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
    if params.tagging.is_some() {
        store.delete_object_tagging(&bucket, &key)?;
        return Ok(StatusCode::NO_CONTENT);
    }

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
