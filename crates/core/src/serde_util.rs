//! Serialization helpers for the UI bridge.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serializer;

/// A relative path as a `/`-separated string.
pub fn rel_path<S: Serializer>(p: &Path, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&crate::format::rel_path(p))
}

/// `Option<SystemTime>` as milliseconds since the Unix epoch (a JS `Date` value), or null.
pub fn opt_time_ms<S: Serializer>(t: &Option<SystemTime>, s: S) -> Result<S::Ok, S::Error> {
    match t.and_then(|t| t.duration_since(UNIX_EPOCH).ok()) {
        Some(d) => s.serialize_some(&(d.as_millis() as f64)),
        None => s.serialize_none(),
    }
}
