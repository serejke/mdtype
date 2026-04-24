//! Validator + Reporter traits and the default `CoreValidator`.

use std::io;

use crate::diagnostic::{Diagnostic, Summary};
use crate::parser::ParsedDocument;
use crate::schema::Schema;

/// Runs a schema against a parsed document and returns diagnostics in a deterministic order.
pub trait Validator {
    /// Validate `doc` against `schema`.
    fn validate(&self, doc: &ParsedDocument, schema: &Schema) -> Vec<Diagnostic>;
}

/// Renders a list of diagnostics. Two built-ins ship: human and JSON.
pub trait Reporter {
    /// Write a rendering of `diagnostics` + `summary` to `out`.
    ///
    /// # Errors
    ///
    /// Propagates any `io::Error` raised by `out`.
    fn report(
        &self,
        diagnostics: &[Diagnostic],
        summary: &Summary,
        out: &mut dyn io::Write,
    ) -> io::Result<()>;
}

/// Default validator: runs the JSON Schema frontmatter check, then each body rule in order,
/// then sorts diagnostics by `(file, line, rule)` for stable output.
pub struct CoreValidator;

impl Validator for CoreValidator {
    fn validate(&self, _doc: &ParsedDocument, _schema: &Schema) -> Vec<Diagnostic> {
        todo!("implemented in Phase 1.4")
    }
}
