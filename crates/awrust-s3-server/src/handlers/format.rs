use awrust_s3_domain::ObjectMeta;
use axum::http::{HeaderMap, HeaderName};
use std::collections::HashMap;
use std::time::{Duration, UNIX_EPOCH};

pub(crate) fn parse_range(header: &str, total: usize) -> Option<(usize, usize)> {
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

pub(crate) fn extract_amz_meta(headers: &HeaderMap) -> HashMap<String, String> {
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

pub(crate) fn meta_to_headers(meta: &ObjectMeta) -> HeaderMap {
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

pub(crate) fn format_iso8601(epoch_secs: u64) -> String {
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
