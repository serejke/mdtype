//! Markdown + frontmatter parser. One path in, `ParsedDocument` out.

use std::path::{Path, PathBuf};

use crate::error::Error;

/// A parsed Markdown file ready for validation.
pub struct ParsedDocument {
    /// Absolute or repo-relative path, preserved for diagnostics.
    pub path: PathBuf,
    /// Parsed frontmatter as a JSON value. `Null` if the file has no frontmatter block.
    pub frontmatter: serde_json::Value,
    /// Raw body text, used by rules that need lines rather than AST nodes.
    pub body: String,
    /// 1-indexed line on which the body starts (i.e., the line after the closing `---`).
    pub body_line_offset: usize,
}

/// Read, split, and parse a Markdown file. Returns a `ParsedDocument` suitable for the validator.
///
/// Phase 1 implementation stub — the real parser will:
/// 1. Detect a leading `---\n...\n---\n` YAML block and parse it into `serde_json::Value`.
/// 2. Feed the remainder into `comrak::parse_document`.
/// 3. Record the body line offset for accurate diagnostic line numbers.
///
/// # Errors
///
/// Returns `Error::Io` on read failures and `Error::Frontmatter` on malformed YAML.
pub fn parse_file(_path: &Path) -> Result<ParsedDocument, Error> {
    todo!("implemented in Phase 1.3")
}
