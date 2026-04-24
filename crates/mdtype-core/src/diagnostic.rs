//! Diagnostics produced by the validator and rules.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A single validation finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Path of the offending file, relative to the config root when possible.
    pub file: PathBuf,
    /// 1-indexed line number. `None` for whole-file issues such as missing frontmatter fields.
    pub line: Option<usize>,
    /// Stable rule identifier, e.g. `"body.forbid_h1"`.
    pub rule: &'static str,
    /// Severity. v1 always emits `Error`.
    pub severity: Severity,
    /// Human-readable description of the issue.
    pub message: String,
    /// Optional machine-consumable fix hint. `mdtype` itself never rewrites files.
    pub fixit: Option<Fixit>,
}

/// Diagnostic severity. v1 emits only `Error`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Violation that blocks acceptance; CLI exits non-zero.
    Error,
    /// Advisory only; does not affect exit code.
    Warning,
}

/// Hint describing how the diagnostic could be repaired. Consumers may act on these.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Fixit {
    /// Add a missing frontmatter field.
    AddFrontmatterField {
        /// Name of the missing field.
        field: String,
        /// Optional free-text hint (e.g., expected type).
        hint: Option<String>,
    },
    /// Delete an offending line (e.g., a forbidden H1).
    DeleteLine {
        /// 1-indexed line number to remove.
        line: usize,
    },
    /// Append a missing section under an optional anchor heading.
    AppendSection {
        /// Heading text to insert, including the `##` markers.
        heading: String,
        /// Heading after which to insert, if any.
        after: Option<String>,
    },
    /// Escape hatch for downstream rules with custom fixits.
    Custom {
        /// Domain-specific fix name, namespaced by the rule (e.g., `"my-rule.rewrite-heading"`).
        name: String,
        /// Free-form payload documented by the rule.
        payload: serde_json::Value,
    },
}

/// Summary statistics rendered by reporters alongside the diagnostic list.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Summary {
    /// Total number of files scanned.
    pub files_scanned: usize,
    /// Number of files that produced at least one diagnostic.
    pub files_with_errors: usize,
    /// Count of diagnostics with `Severity::Error`.
    pub errors: usize,
    /// Count of diagnostics with `Severity::Warning`.
    pub warnings: usize,
}
