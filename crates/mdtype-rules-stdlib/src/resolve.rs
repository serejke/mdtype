//! Shared filesystem-resolution helpers for cross-file rules.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Build a canonical-path → walked-path index over the workspace files.
///
/// Files whose canonicalisation fails (e.g. a path that no longer exists by the time
/// the rule runs) are silently dropped — workspace rules report missing targets as
/// their own diagnostic class, so an unresolvable entry would only produce noise.
pub fn build_canonical_index(files: &[PathBuf]) -> HashMap<PathBuf, PathBuf> {
    let mut map: HashMap<PathBuf, PathBuf> = HashMap::with_capacity(files.len());
    for file in files {
        if let Ok(canonical) = fs::canonicalize(file) {
            map.insert(canonical, file.clone());
        }
    }
    map
}
