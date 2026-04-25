//! Validator + Reporter traits and the default `CoreValidator`.

use std::io;

use crate::diagnostic::{Diagnostic, Severity, Summary};
use crate::parser::ParsedDocument;
use crate::schema::Schema;

/// Stable rule id for any frontmatter JSON Schema violation.
pub const FRONTMATTER_RULE_ID: &str = "frontmatter.schema";

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
    fn validate(&self, doc: &ParsedDocument, schema: &Schema) -> Vec<Diagnostic> {
        let mut out: Vec<Diagnostic> = Vec::new();

        if let Some(fm_schema) = schema.frontmatter.as_ref() {
            match jsonschema::draft202012::new(fm_schema) {
                Ok(validator) => {
                    if let Err(errors) = validator.validate(&doc.frontmatter) {
                        for err in errors {
                            out.push(Diagnostic {
                                file: doc.path.clone(),
                                line: None,
                                rule: FRONTMATTER_RULE_ID,
                                severity: Severity::Error,
                                message: format_schema_error(&err),
                                fixit: None,
                            });
                        }
                    }
                }
                Err(e) => {
                    out.push(Diagnostic {
                        file: doc.path.clone(),
                        line: None,
                        rule: FRONTMATTER_RULE_ID,
                        severity: Severity::Error,
                        message: format!("invalid JSON Schema for frontmatter: {e}"),
                        fixit: None,
                    });
                }
            }
        }

        for rule in &schema.body {
            rule.check(doc, &mut out);
        }

        out.sort_by(|a, b| {
            a.file
                .cmp(&b.file)
                .then_with(|| a.line.cmp(&b.line))
                .then_with(|| a.rule.cmp(b.rule))
        });

        out
    }
}

fn format_schema_error(err: &jsonschema::ValidationError<'_>) -> String {
    use jsonschema::error::{TypeKind, ValidationErrorKind};

    let path = err.instance_path.to_string();
    let path_or_root: &str = if path.is_empty() { "/" } else { &path };

    match &err.kind {
        ValidationErrorKind::Required { property } => {
            let name = property.as_str().unwrap_or("<unknown>");
            format!("missing required field '{name}'")
        }
        ValidationErrorKind::AdditionalProperties { unexpected } => {
            let names = unexpected
                .iter()
                .map(|s| format!("'{s}'"))
                .collect::<Vec<_>>()
                .join(", ");
            format!("unexpected field {names} (schema declares additionalProperties=false)")
        }
        ValidationErrorKind::Type { kind } => {
            let expected = match kind {
                TypeKind::Single(t) => format!("{t:?}").to_lowercase(),
                TypeKind::Multiple(_) => "one of the declared types".to_string(),
            };
            let found = describe_value(&err.instance);
            if path.is_empty() {
                format!("frontmatter: expected {expected}, found {found}")
            } else {
                format!("field '{path_or_root}': expected {expected}, found {found}")
            }
        }
        _ => {
            // Fall through to jsonschema's own message, prefixed with the JSON pointer when known.
            if path.is_empty() {
                err.to_string()
            } else {
                format!("field '{path_or_root}': {err}")
            }
        }
    }
}

/// One-word description of a JSON value used in type-mismatch error messages.
fn describe_value(value: &serde_json::Value) -> String {
    use serde_json::Value;
    match value {
        Value::Null => "null".into(),
        Value::Bool(_) => "boolean".into(),
        Value::Number(_) => "number".into(),
        Value::String(s) => format!("\"{s}\""),
        Value::Array(_) => "array".into(),
        Value::Object(_) => "object".into(),
    }
}

#[cfg(test)]
mod tests {
    use comrak::Arena;
    use serde_json::json;
    use std::io::Write;
    use tempfile::NamedTempFile;

    use super::{CoreValidator, Validator, FRONTMATTER_RULE_ID};
    use crate::parser::parse_file;
    use crate::schema::Schema;

    fn write_tmp(contents: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().expect("tempfile");
        f.write_all(contents.as_bytes()).expect("write");
        f
    }

    #[test]
    fn empty_schema_produces_no_diagnostics() {
        let f = write_tmp("---\ntitle: Hi\n---\n# Body\n");
        let arena = Arena::new();
        let doc = parse_file(f.path(), &arena).expect("parse");

        let schema = Schema {
            name: "empty".into(),
            description: None,
            frontmatter: None,
            body: Vec::new(),
            workspace: Vec::new(),
        };

        let diagnostics = CoreValidator.validate(&doc, &schema);
        assert!(
            diagnostics.is_empty(),
            "expected zero diagnostics, got {diagnostics:?}"
        );
    }

    #[test]
    fn missing_required_frontmatter_field_diagnoses() {
        let f = write_tmp("---\ntitle: Hi\n---\n# Body\n");
        let arena = Arena::new();
        let doc = parse_file(f.path(), &arena).expect("parse");

        let schema = Schema {
            name: "needs-author".into(),
            description: None,
            frontmatter: Some(json!({
                "type": "object",
                "required": ["title", "author"],
                "properties": {
                    "title": { "type": "string" },
                    "author": { "type": "string" }
                }
            })),
            body: Vec::new(),
            workspace: Vec::new(),
        };

        let diagnostics = CoreValidator.validate(&doc, &schema);
        assert_eq!(
            diagnostics.len(),
            1,
            "expected one diagnostic, got {diagnostics:?}"
        );
        let d = &diagnostics[0];
        assert_eq!(d.rule, FRONTMATTER_RULE_ID);
        assert!(d.line.is_none());
        assert!(
            d.message.contains("author"),
            "message should mention the missing field: {}",
            d.message
        );
    }
}
