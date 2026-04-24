//! Markdown + frontmatter parser. One path in, `ParsedDocument` out.

use std::path::{Path, PathBuf};

use comrak::nodes::AstNode;
use comrak::Arena;

use crate::error::Error;

/// A parsed Markdown file ready for validation.
///
/// The AST is borrowed from a caller-supplied [`Arena`] so its lifetime is bound to that
/// arena. Callers typically own the arena for the duration of validation:
///
/// ```ignore
/// let arena = comrak::Arena::new();
/// let doc = mdtype_core::parse_file(path, &arena)?;
/// validator.validate(&doc, &schema);
/// ```
pub struct ParsedDocument<'a> {
    /// Absolute or repo-relative path, preserved for diagnostics.
    pub path: PathBuf,
    /// Parsed frontmatter as a JSON value. `Null` if the file has no frontmatter block.
    pub frontmatter: serde_json::Value,
    /// Root node of the `CommonMark` AST produced by `comrak`.
    pub ast: &'a AstNode<'a>,
    /// 1-indexed line on which the body starts (i.e., the line after the closing `---`).
    /// `1` if the file has no frontmatter block.
    pub body_line_offset: usize,
}

/// Read, split, and parse a Markdown file. Returns a `ParsedDocument` suitable for the validator.
///
/// Phase 1 implementation stub — the real parser will:
/// 1. Detect a leading `---\n...\n---\n` YAML block and parse it into `serde_json::Value`.
/// 2. Feed the remainder into `comrak::parse_document`, allocating nodes in `arena`.
/// 3. Record the body line offset for accurate diagnostic line numbers.
///
/// # Errors
///
/// Returns `Error::Io` on read failures and `Error::Frontmatter` on malformed YAML.
pub fn parse_file<'a>(
    _path: &Path,
    _arena: &'a Arena<AstNode<'a>>,
) -> Result<ParsedDocument<'a>, Error> {
    todo!("implemented in Phase 1.3")
}
