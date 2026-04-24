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
/// 1. Detect a leading `---\n...\n---\n` YAML block and parse it into [`serde_json::Value`].
/// 2. Feed the remainder into `comrak::parse_document`, allocating nodes in `arena`.
/// 3. Record the body line offset so rule diagnostics can report absolute line numbers.
///
/// # Errors
///
/// Returns [`Error::Io`] on read failures and [`Error::Frontmatter`] on a malformed or
/// unterminated YAML block.
pub fn parse_file<'a>(
    path: &Path,
    arena: &'a Arena<AstNode<'a>>,
) -> Result<ParsedDocument<'a>, Error> {
    let raw = std::fs::read_to_string(path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let (frontmatter, body, body_line_offset) = split_frontmatter(&raw, path)?;
    let ast = comrak::parse_document(arena, &body, &comrak::Options::default());
    Ok(ParsedDocument {
        path: path.to_path_buf(),
        frontmatter,
        ast,
        body_line_offset,
    })
}

/// Split a raw Markdown string into `(frontmatter, body, body_line_offset)`.
///
/// `body_line_offset` is the 1-indexed line number of the first body line in the original
/// source — i.e., the line after the closing `---`. `1` when there is no frontmatter block.
fn split_frontmatter(
    raw: &str,
    path: &Path,
) -> Result<(serde_json::Value, String, usize), Error> {
    let mut iter = raw.split_inclusive('\n');
    let Some(first) = iter.next() else {
        return Ok((serde_json::Value::Null, String::new(), 1));
    };
    if first.trim_end_matches(['\r', '\n']) != "---" {
        return Ok((serde_json::Value::Null, raw.to_string(), 1));
    }

    let mut yaml = String::new();
    let mut closing_line: Option<usize> = None;
    let mut current_line = 1_usize;
    for line in iter.by_ref() {
        current_line += 1;
        if line.trim_end_matches(['\r', '\n']) == "---" {
            closing_line = Some(current_line);
            break;
        }
        yaml.push_str(line);
    }

    let Some(closing_line) = closing_line else {
        return Err(Error::Frontmatter {
            path: path.to_path_buf(),
            message: "missing closing `---` fence".into(),
        });
    };

    let frontmatter = if yaml.trim().is_empty() {
        serde_json::Value::Null
    } else {
        let value: serde_yaml::Value =
            serde_yaml::from_str(&yaml).map_err(|e| Error::Frontmatter {
                path: path.to_path_buf(),
                message: e.to_string(),
            })?;
        serde_json::to_value(&value).map_err(|e| Error::Frontmatter {
            path: path.to_path_buf(),
            message: format!("yaml→json conversion: {e}"),
        })?
    };

    let body: String = iter.collect();
    Ok((frontmatter, body, closing_line + 1))
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use comrak::Arena;
    use tempfile::NamedTempFile;

    use super::parse_file;

    fn write_tmp(contents: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().expect("tempfile");
        f.write_all(contents.as_bytes()).expect("write");
        f
    }

    #[test]
    fn parses_file_with_frontmatter() {
        let src = "---\ntitle: Hello\ntags: [a, b]\n---\n# Body\n\nText.\n";
        let f = write_tmp(src);
        let arena = Arena::new();
        let doc = parse_file(f.path(), &arena).expect("parse");

        assert_eq!(doc.frontmatter["title"], serde_json::json!("Hello"));
        assert_eq!(doc.frontmatter["tags"], serde_json::json!(["a", "b"]));
        assert_eq!(doc.body_line_offset, 5);
        // AST root is a Document with at least one child (the H1).
        assert!(doc.ast.first_child().is_some());
    }

    #[test]
    fn parses_file_without_frontmatter() {
        let src = "# Just a body\n\nNo frontmatter here.\n";
        let f = write_tmp(src);
        let arena = Arena::new();
        let doc = parse_file(f.path(), &arena).expect("parse");

        assert!(doc.frontmatter.is_null());
        assert_eq!(doc.body_line_offset, 1);
        assert!(doc.ast.first_child().is_some());
    }

    #[test]
    fn unterminated_frontmatter_errors() {
        let src = "---\ntitle: oops\n# Body that never closes\n";
        let f = write_tmp(src);
        let arena = Arena::new();
        let result = parse_file(f.path(), &arena);
        match result {
            Err(crate::Error::Frontmatter { .. }) => {}
            Err(other) => panic!("expected Frontmatter error, got {other:?}"),
            Ok(_) => panic!("expected error, got Ok"),
        }
    }
}
