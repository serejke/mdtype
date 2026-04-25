//! `links.relative_path` — resolve inline Markdown links against the source file's
//! directory and report broken targets and broken anchors.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use mdtype_core::{
    Diagnostic, Error, LinkKind, Requirements, Severity, Workspace, WorkspaceRule,
    WorkspaceRuleFactory,
};
use serde::Deserialize;

/// Stable rule id, exposed for downstream crates to reference.
pub const ID: &str = "links.relative_path";

const DEFAULT_IGNORE_SCHEMES: &[&str] = &["http", "https", "mailto", "tel"];

/// Inline-link integrity check.
pub struct Rule {
    /// Schemes whose links are skipped (e.g. `http`, `mailto`). Lower-cased on build.
    ignore_schemes: HashSet<String>,
    /// When true, validate `#anchor` fragments against the target file's heading slugs.
    check_anchors: bool,
}

impl WorkspaceRule for Rule {
    fn id(&self) -> &'static str {
        ID
    }

    fn requires(&self) -> Requirements {
        Requirements {
            headings: self.check_anchors,
            links_inline: true,
            ..Requirements::default()
        }
    }

    fn check(&self, ws: &Workspace, scope: &[PathBuf], out: &mut Vec<Diagnostic>) {
        let canonical = build_canonical_index(&ws.files);

        for source in scope {
            let Some(links) = ws.links.get(source) else {
                continue;
            };
            let Some(parent) = source.parent() else {
                continue;
            };
            for link in links {
                if !matches!(link.kind, LinkKind::Inline) {
                    continue;
                }
                if let Some(scheme) = extract_scheme(&link.target) {
                    if self.ignore_schemes.contains(&scheme.to_ascii_lowercase()) {
                        continue;
                    }
                }
                if link.target.is_empty() {
                    if self.check_anchors {
                        if let Some(anchor) = link.anchor.as_deref() {
                            if !heading_has_slug(ws.headings.get(source), anchor) {
                                out.push(missing_anchor_diagnostic(source, link.line, anchor));
                            }
                        }
                    }
                    continue;
                }

                let resolved = parent.join(&link.target);
                let Ok(canonical_target) = fs::canonicalize(&resolved) else {
                    out.push(missing_target_diagnostic(source, link.line, &link.target));
                    continue;
                };
                let Some(original) = canonical.get(&canonical_target) else {
                    out.push(missing_target_diagnostic(source, link.line, &link.target));
                    continue;
                };

                if self.check_anchors {
                    if let Some(anchor) = link.anchor.as_deref() {
                        if !heading_has_slug(ws.headings.get(original), anchor) {
                            out.push(missing_anchor_diagnostic_for(
                                source,
                                link.line,
                                &link.target,
                                anchor,
                            ));
                        }
                    }
                }
            }
        }
    }
}

/// Factory that builds [`Rule`] from YAML params.
pub struct Factory;

impl WorkspaceRuleFactory for Factory {
    fn id(&self) -> &'static str {
        ID
    }

    fn build(&self, params: &serde_json::Value) -> Result<Box<dyn WorkspaceRule>, Error> {
        let raw: Params = if params.is_null() {
            Params::default()
        } else {
            serde_json::from_value(params.clone())
                .map_err(|e| Error::Schema(format!("invalid params for {ID}: {e}")))?
        };
        let ignore_schemes = raw
            .ignore_schemes
            .unwrap_or_else(|| {
                DEFAULT_IGNORE_SCHEMES
                    .iter()
                    .map(|s| (*s).to_string())
                    .collect()
            })
            .into_iter()
            .map(|s| s.to_ascii_lowercase())
            .collect();
        Ok(Box::new(Rule {
            ignore_schemes,
            check_anchors: raw.check_anchors.unwrap_or(true),
        }))
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Params {
    #[serde(default)]
    ignore_schemes: Option<Vec<String>>,
    #[serde(default)]
    check_anchors: Option<bool>,
}

fn build_canonical_index(files: &[PathBuf]) -> HashMap<PathBuf, PathBuf> {
    let mut map: HashMap<PathBuf, PathBuf> = HashMap::with_capacity(files.len());
    for file in files {
        if let Ok(canonical) = fs::canonicalize(file) {
            map.insert(canonical, file.clone());
        }
    }
    map
}

fn extract_scheme(target: &str) -> Option<&str> {
    let bytes = target.as_bytes();
    let first = *bytes.first()?;
    if !first.is_ascii_alphabetic() {
        return None;
    }
    for (i, b) in bytes.iter().enumerate().skip(1) {
        if *b == b':' {
            return Some(&target[..i]);
        }
        if !(b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.')) {
            return None;
        }
    }
    None
}

fn heading_has_slug(headings: Option<&Vec<mdtype_core::HeadingFact>>, anchor: &str) -> bool {
    headings.is_some_and(|hs| hs.iter().any(|h| h.slug == anchor))
}

fn missing_target_diagnostic(source: &Path, line: usize, target: &str) -> Diagnostic {
    Diagnostic {
        file: source.to_path_buf(),
        line: Some(line),
        rule: ID,
        severity: Severity::Error,
        message: format!("link target '{target}' not found in workspace"),
        fixit: None,
    }
}

fn missing_anchor_diagnostic(source: &Path, line: usize, anchor: &str) -> Diagnostic {
    Diagnostic {
        file: source.to_path_buf(),
        line: Some(line),
        rule: ID,
        severity: Severity::Error,
        message: format!("anchor '#{anchor}' has no matching heading"),
        fixit: None,
    }
}

fn missing_anchor_diagnostic_for(
    source: &Path,
    line: usize,
    target: &str,
    anchor: &str,
) -> Diagnostic {
    Diagnostic {
        file: source.to_path_buf(),
        line: Some(line),
        rule: ID,
        severity: Severity::Error,
        message: format!("link target '{target}' has no heading matching anchor '#{anchor}'"),
        fixit: None,
    }
}

#[cfg(test)]
mod tests {
    use super::extract_scheme;

    #[test]
    fn scheme_detection_handles_typical_cases() {
        assert_eq!(extract_scheme("https://example.com"), Some("https"));
        assert_eq!(extract_scheme("mailto:a@b"), Some("mailto"));
        assert_eq!(extract_scheme("./relative.md"), None);
        assert_eq!(extract_scheme("../sibling.md"), None);
        assert_eq!(extract_scheme("file.md"), None);
        assert_eq!(extract_scheme("file.md#section"), None);
        assert_eq!(extract_scheme(""), None);
        assert_eq!(extract_scheme("9foo:bar"), None);
    }
}
