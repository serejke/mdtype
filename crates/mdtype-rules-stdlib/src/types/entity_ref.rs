//! `types.entity_ref` — type-check frontmatter reference fields against the target
//! file's declared entity.
//!
//! This module is **not** a user-enabled workspace rule. The check is declaration-driven:
//! a schema's frontmatter JSON Schema carries `x-entity` annotations on string-typed
//! properties or array items; the schema loader walks those annotations into a
//! [`ReferenceSpec`] vector; [`crate::install_type_checks`] then synthesises a
//! [`WorkspaceRule`] instance from the specs and pushes it into the schema's `workspace`
//! pipeline. From the user's perspective the rule never appears in YAML.
//!
//! See `docs/types.md`.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use mdtype_core::{Diagnostic, ReferenceSpec, Requirements, Severity, Workspace, WorkspaceRule};

use crate::resolve::build_canonical_index;

/// Stable diagnostic id. Surfaces on every diagnostic this rule emits.
pub const ID: &str = "types.entity_ref";

/// Construct a workspace-rule instance from a schema's reference specs.
///
/// Used by [`crate::install_type_checks`] after the schema loader has walked the
/// frontmatter JSON Schema for `x-entity` annotations.
#[must_use]
pub fn build(specs: Vec<ReferenceSpec>) -> Box<dyn WorkspaceRule> {
    Box::new(Rule { specs })
}

struct Rule {
    specs: Vec<ReferenceSpec>,
}

impl WorkspaceRule for Rule {
    fn id(&self) -> &'static str {
        ID
    }

    fn requires(&self) -> Requirements {
        // Frontmatter and entities are always populated by the runner — no flags needed.
        Requirements::default()
    }

    fn check(&self, ws: &Workspace, scope: &[PathBuf], out: &mut Vec<Diagnostic>) {
        let canonical = build_canonical_index(&ws.files);

        for source in scope {
            let Some(fm) = ws.frontmatter.get(source) else {
                continue;
            };
            let Some(parent) = source.parent() else {
                continue;
            };
            for spec in &self.specs {
                judge_field(source, parent, fm, spec, &canonical, ws, out);
            }
        }
    }
}

fn judge_field(
    source: &Path,
    parent: &Path,
    fm: &serde_json::Value,
    spec: &ReferenceSpec,
    canonical: &HashMap<PathBuf, PathBuf>,
    ws: &Workspace,
    out: &mut Vec<Diagnostic>,
) {
    use serde_json::Value;
    let Some(value) = fm.get(&spec.field) else {
        return;
    };
    match value {
        Value::Null => {}
        Value::String(s) => {
            judge_one_value(source, parent, s, spec, canonical, ws, out);
        }
        Value::Array(items) => {
            if items.iter().any(|item| !item.is_string()) {
                out.push(field_invalid(source, &spec.field, "array of mixed types"));
                return;
            }
            for item in items {
                if let Some(s) = item.as_str() {
                    judge_one_value(source, parent, s, spec, canonical, ws, out);
                }
            }
        }
        Value::Bool(_) => out.push(field_invalid(source, &spec.field, "boolean")),
        Value::Number(_) => out.push(field_invalid(source, &spec.field, "number")),
        Value::Object(_) => out.push(field_invalid(source, &spec.field, "object")),
    }
}

fn judge_one_value(
    source: &Path,
    parent: &Path,
    raw: &str,
    spec: &ReferenceSpec,
    canonical: &HashMap<PathBuf, PathBuf>,
    ws: &Workspace,
    out: &mut Vec<Diagnostic>,
) {
    if raw.contains('#') {
        out.push(target_anchor_unsupported(source, &spec.field, raw));
        return;
    }

    let resolved = parent.join(raw);
    let Ok(canonical_target) = fs::canonicalize(&resolved) else {
        out.push(target_missing(source, &spec.field, raw));
        return;
    };
    let Some(walked) = canonical.get(&canonical_target) else {
        out.push(target_missing(source, &spec.field, raw));
        return;
    };
    match ws.entities.get(walked) {
        None => out.push(target_untyped(source, &spec.field, raw, &spec.targets)),
        Some(actual) if !spec.targets.iter().any(|t| t == actual) => {
            out.push(target_type(source, &spec.field, raw, &spec.targets, actual));
        }
        Some(_) => {}
    }
}

fn render_expected(targets: &[String]) -> String {
    if targets.len() == 1 {
        format!("'{}'", targets[0])
    } else {
        let quoted: Vec<String> = targets.iter().map(|t| format!("'{t}'")).collect();
        format!("one of {}", quoted.join(", "))
    }
}

fn field_invalid(source: &Path, field: &str, kind: &str) -> Diagnostic {
    Diagnostic {
        file: source.to_path_buf(),
        line: None,
        rule: ID,
        severity: Severity::Error,
        message: format!("field '{field}': expected string or array of strings, found {kind}"),
        fixit: None,
    }
}

fn target_anchor_unsupported(source: &Path, field: &str, target: &str) -> Diagnostic {
    Diagnostic {
        file: source.to_path_buf(),
        line: None,
        rule: ID,
        severity: Severity::Error,
        message: format!(
            "field '{field}': link target '{target}' carries an anchor; entity references must be document-level"
        ),
        fixit: None,
    }
}

fn target_missing(source: &Path, field: &str, target: &str) -> Diagnostic {
    Diagnostic {
        file: source.to_path_buf(),
        line: None,
        rule: ID,
        severity: Severity::Error,
        message: format!("field '{field}': link target '{target}' not found in workspace"),
        fixit: None,
    }
}

fn target_untyped(source: &Path, field: &str, target: &str, expected: &[String]) -> Diagnostic {
    Diagnostic {
        file: source.to_path_buf(),
        line: None,
        rule: ID,
        severity: Severity::Error,
        message: format!(
            "field '{field}': link target '{target}' has no declared entity, expected {}",
            render_expected(expected)
        ),
        fixit: None,
    }
}

fn target_type(
    source: &Path,
    field: &str,
    target: &str,
    expected: &[String],
    actual: &str,
) -> Diagnostic {
    Diagnostic {
        file: source.to_path_buf(),
        line: None,
        rule: ID,
        severity: Severity::Error,
        message: format!(
            "field '{field}': link target '{target}': expected entity {}, got '{actual}'",
            render_expected(expected)
        ),
        fixit: None,
    }
}

#[cfg(test)]
mod tests {
    use super::render_expected;

    #[test]
    fn render_expected_single() {
        assert_eq!(render_expected(&["author".into()]), "'author'");
    }

    #[test]
    fn render_expected_union_two() {
        assert_eq!(
            render_expected(&["meeting-transcript".into(), "adr".into()]),
            "one of 'meeting-transcript', 'adr'"
        );
    }

    #[test]
    fn render_expected_union_three() {
        assert_eq!(
            render_expected(&["a".into(), "b".into(), "c".into()]),
            "one of 'a', 'b', 'c'"
        );
    }
}
