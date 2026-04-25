//! `body.forbid_h1` — disallow any top-level `#` heading.

use mdtype_core::nodes::NodeValue;
use mdtype_core::{BodyRule, BodyRuleFactory, Diagnostic, Error, ParsedDocument, Severity};

/// Rule id, exposed as a constant for downstream crates to reference.
pub const ID: &str = "body.forbid_h1";

/// The rule itself. Stateless — no configuration.
pub struct Rule;

impl BodyRule for Rule {
    fn id(&self) -> &'static str {
        ID
    }

    fn check(&self, doc: &ParsedDocument, out: &mut Vec<Diagnostic>) {
        for node in doc.ast.descendants() {
            let data = node.data.borrow();
            let NodeValue::Heading(heading) = &data.value else {
                continue;
            };
            if heading.level != 1 {
                continue;
            }
            let absolute_line = data.sourcepos.start.line + doc.body_line_offset.saturating_sub(1);
            let text = crate::heading_text(node);
            out.push(Diagnostic {
                file: doc.path.clone(),
                line: Some(absolute_line),
                rule: ID,
                severity: Severity::Error,
                message: format!("top-level heading '# {text}' is not allowed; use '## {text}' or rely on the file title"),
                fixit: Some(mdtype_core::Fixit::DeleteLine {
                    line: absolute_line,
                }),
            });
        }
    }
}

/// Factory that builds `Rule` from (empty) YAML parameters.
pub struct Factory;

impl BodyRuleFactory for Factory {
    fn id(&self) -> &'static str {
        ID
    }

    fn build(&self, _params: &serde_json::Value) -> Result<Box<dyn BodyRule>, Error> {
        Ok(Box::new(Rule))
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use mdtype_core::{comrak, Arena, BodyRule, ParsedDocument};

    use super::{Rule, ID};

    fn doc_for<'a>(
        arena: &'a Arena<mdtype_core::nodes::AstNode<'a>>,
        body: &str,
    ) -> ParsedDocument<'a> {
        let ast = comrak::parse_document(arena, body, &comrak::Options::default());
        ParsedDocument {
            path: PathBuf::from("fixture.md"),
            frontmatter: serde_json::Value::Null,
            ast,
            body_line_offset: 1,
        }
    }

    #[test]
    fn h1_present_emits_diagnostic_with_line() {
        let arena = Arena::new();
        let body = "Some intro text.\n\n# A stray H1\n\n## Summary\n\nMore body.\n";
        let doc = doc_for(&arena, body);

        let mut diags = Vec::new();
        Rule.check(&doc, &mut diags);

        assert_eq!(diags.len(), 1, "expected one diagnostic, got {diags:?}");
        let d = &diags[0];
        assert_eq!(d.rule, ID);
        assert_eq!(d.line, Some(3));
        assert!(
            d.message.contains("H1"),
            "message should mention H1: {}",
            d.message
        );
    }

    #[test]
    fn h1_absent_emits_nothing() {
        let arena = Arena::new();
        let body = "## Summary\n\nNo H1 here, only H2 and below.\n\n### Detail\n";
        let doc = doc_for(&arena, body);

        let mut diags = Vec::new();
        Rule.check(&doc, &mut diags);

        assert!(diags.is_empty(), "expected no diagnostics, got {diags:?}");
    }

    #[test]
    fn body_line_offset_is_added_for_files_with_frontmatter() {
        let arena = Arena::new();
        let body = "# H1 on body line 1\n";
        let ast = comrak::parse_document(&arena, body, &comrak::Options::default());
        let doc = ParsedDocument {
            path: PathBuf::from("fixture.md"),
            frontmatter: serde_json::Value::Null,
            ast,
            // Frontmatter took lines 1-4; body starts at line 5.
            body_line_offset: 5,
        };

        let mut diags = Vec::new();
        Rule.check(&doc, &mut diags);

        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].line, Some(5));
    }
}
