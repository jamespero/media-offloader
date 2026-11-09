//! What a history holds.

use std::path::PathBuf;

use serde::Serialize;

use super::HashFormat;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HashValue {
    pub format: HashFormat,
    pub value: String,
    pub action: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    /// `/`-separated, relative to the history root.
    pub path: String,
    pub size: Option<u64>,
    pub hashes: Vec<HashValue>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Generation {
    pub number: u32,
    pub file_name: String,
    /// Whether the manifest file still matches the C4 recorded in the chain
    /// (`None` when the chain doesn't list it).
    pub c4_ok: Option<bool>,
    pub process: Option<String>,
    pub tool: Option<String>,
    pub creation_date: Option<String>,
    pub ignore: Vec<String>,
    pub entries: Vec<Entry>,
}

/// An `ascmhl` history: every generation, oldest first.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct History {
    pub root: PathBuf,
    pub generations: Vec<Generation>,
    /// Damaged or missing manifests, chain problems.
    pub problems: Vec<String>,
}

/// What a file should hash to, according to a history or a `.md5` list.
#[derive(Clone, Debug)]
pub struct Expected {
    pub size: Option<u64>,
    pub hashes: Vec<(HashFormat, String)>,
}
