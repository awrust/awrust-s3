use awrust_s3_domain::{ListObjectsParams, Store};
use axum::body::Bytes;
use axum::extract::{Path, Query, RawQuery, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use std::sync::Arc;

use super::format::format_iso8601;
use super::{BucketQueryParams, ListParams, S3Result};
use crate::xml::{
    BucketEntry, BucketList, CommonPrefix, DeleteErrorEntry, DeleteResult, DeletedEntry,
    ListAllMyBucketsResult, ListBucketResult, ListMultipartUploadsResult, ListVersionsResult,
    LocationConstraint, ObjectEntry, UploadEntry, VersionEntry, XmlResponse,
};

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

async fn list_objects(
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

    if params.versions.is_some() {
        return list_object_versions(store, &bucket, &params).await;
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

async fn list_object_versions(
    store: Arc<dyn Store>,
    bucket: &str,
    params: &ListParams,
) -> S3Result<Response> {
    let max_keys = params.max_keys.unwrap_or(1000);
    let page = store.list_objects(
        bucket,
        &ListObjectsParams {
            prefix: params.prefix.clone(),
            delimiter: None,
            continuation_token: None,
            max_keys,
        },
    )?;

    let result = ListVersionsResult {
        name: bucket.to_owned(),
        prefix: params.prefix.clone().unwrap_or_default(),
        key_marker: String::new(),
        version_id_marker: String::new(),
        max_keys,
        is_truncated: page.is_truncated,
        versions: page
            .objects
            .into_iter()
            .map(|o| VersionEntry {
                key: o.key,
                version_id: "null".to_owned(),
                is_latest: true,
                last_modified: format_iso8601(o.last_modified),
                etag: o.etag,
                size: o.size,
                storage_class: "STANDARD".to_owned(),
            })
            .collect(),
    };

    Ok(XmlResponse(result).into_response())
}
