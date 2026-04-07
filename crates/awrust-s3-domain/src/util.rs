use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use md5::{Digest, Md5};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::types::{ListObjectsPage, ObjectSummary};

pub(crate) fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time after epoch")
        .as_secs()
}

pub(crate) fn composite_etag(part_digests: &[Vec<u8>]) -> String {
    let mut hasher = Md5::new();
    for digest in part_digests {
        hasher.update(digest);
    }
    format!("\"{:x}-{}\"", hasher.finalize(), part_digests.len())
}

pub(crate) fn apply_delimiter(
    sorted: Vec<ObjectSummary>,
    prefix: &str,
    delimiter: Option<&str>,
    max_keys: usize,
) -> ListObjectsPage {
    let Some(delim) = delimiter else {
        let is_truncated = sorted.len() > max_keys;
        let mut objects = sorted;
        objects.truncate(max_keys);
        let next_continuation_token = if is_truncated {
            objects.last().map(|s| encode_continuation_token(&s.key))
        } else {
            None
        };
        return ListObjectsPage {
            objects,
            common_prefixes: Vec::new(),
            is_truncated,
            next_continuation_token,
        };
    };

    let mut objects = Vec::new();
    let mut prefixes = std::collections::BTreeSet::new();
    let mut last_key_seen = String::new();

    for obj in &sorted {
        last_key_seen.clone_from(&obj.key);
        let after = &obj.key[prefix.len()..];
        if let Some(pos) = after.find(delim) {
            let cp = format!("{}{}", prefix, &after[..pos + delim.len()]);
            let is_new = prefixes.insert(cp);
            if is_new && objects.len() + prefixes.len() > max_keys {
                break;
            }
        } else {
            objects.push(obj.clone());
            if objects.len() + prefixes.len() > max_keys {
                break;
            }
        }
    }

    let total = objects.len() + prefixes.len();
    let is_truncated = total > max_keys;
    objects.truncate(max_keys.saturating_sub(prefixes.len()));
    let common_prefixes: Vec<String> = prefixes
        .into_iter()
        .take(max_keys - objects.len())
        .collect();

    let next_continuation_token = if is_truncated {
        Some(encode_continuation_token(&last_key_seen))
    } else {
        None
    };

    ListObjectsPage {
        objects,
        common_prefixes,
        is_truncated,
        next_continuation_token,
    }
}

pub(crate) fn encode_continuation_token(key: &str) -> String {
    BASE64.encode(key.as_bytes())
}

pub(crate) fn decode_continuation_token(token: &str) -> Option<String> {
    BASE64
        .decode(token)
        .ok()
        .and_then(|b| String::from_utf8(b).ok())
}
