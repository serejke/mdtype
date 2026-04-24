//! `body.forbidden_sections` — assert that named H2 headings do not appear.

use mdtype_core::nodes::{AstNode, NodeValue};
use mdtype_core::{BodyRule, BodyRuleFactory, Diagnostic, Error, Fixit, ParsedDocument, Severity};
use serde::Deserialize;

/// Rule id.
pub const ID: &str = "body.forbidden_sections";

/// Configured rule instance.
pub struct Rule {
    /// Exact H2 heading texts that must not appear.
    pub sections: Vec<String>,
}

impl BodyRule for Rule {
    fn id(&self) -> &'static str {
        ID
    }

    fn check(&self, doc: &ParsedDocument, out: &mut Vec<Diagnostic>) {
        let body_offset = doc.body_line_offset.saturating_sub(1);
        for node in doc.ast.descendants() {
            let data = node.data.borrow();
            let NodeValue::Heading(h) = &data.value else {
                continue;
            };
            if h.level != 2 {
                continue;
            }
            let text = heading_text(node);
            if self.sections.iter().any(|s| s == &text) {
                let absolute = data.sourcepos.start.line + body_offset;
                out.push(Diagnostic {
                    file: doc.path.clone(),
                    line: Some(absolute),
                    rule: ID,
                    severity: Severity::Error,
                    message: format!("forbidden section '{text}'"),
                    fixit: Some(Fixit::DeleteLine { line: absolute }),
                });
            }
        }
    }
}

fn heading_text<'a>(heading: &'a AstNode<'a>) -> String {
    let mut buf = String::new();
    for desc in heading.descendants().skip(1) {
        let data = desc.data.borrow();
        match &data.value {
            NodeValue::Text(t) => buf.push_str(t),
            NodeValue::Code(c) => buf.push_str(&c.literal),
            _ => {}
        }
    }
    buf
}

/// Factory. Params shape: `{ sections: [String, ...] }`.
pub struct Factory;

impl BodyRuleFactory for Factory {
    fn id(&self) -> &'static str {
        ID
    }

    fn build(&self, params: &serde_json::Value) -> Result<Box<dyn BodyRule>, Error> {
        let parsed: Params = serde_json::from_value(params.clone())
            .map_err(|e| Error::Schema(format!("{ID}: invalid params: {e}")))?;
        if parsed.sections.is_empty() {
            return Err(Error::Schema(format!("{ID}: `sections` must not be empty")));
        }
        Ok(Box::new(Rule {
            sections: parsed.sections,
        }))
    }
}

#[derive(Debug, Deserialize)]
struct Params {
    sections: Vec<String>,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use mdtype_core::{comrak, Arena, BodyRule, BodyRuleFactory, ParsedDocument};
    use serde_json::json;

    use super::{Factory, Rule, ID};

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
    fn forbidden_present_emits_diagnostic_with_line() {
        let arena = Arena::new();
        let body = "## Summary\n\nx\n\n## TODO\n\nremember to do this\n";
        let doc = doc_for(&arena, body);
        let rule = Rule {
            sections: vec!["TODO".into(), "Scratch".into()],
        };
        let mut diags = Vec::new();
        rule.check(&doc, &mut diags);
        assert_eq!(diags.len(), 1);
        let d = &diags[0];
        assert_eq!(d.rule, ID);
        assert_eq!(d.line, Some(5));
        assert!(d.message.contains("TODO"));
    }

    #[test]
    fn forbidden_absent_is_clean() {
        let arena = Arena::new();
        let body = "## Summary\n\nx\n\n## Conclusion\n\nz\n";
        let doc = doc_for(&arena, body);
        let rule = Rule {
            sections: vec!["TODO".into(), "Scratch".into()],
        };
        let mut diags = Vec::new();
        rule.check(&doc, &mut diags);
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn multiple_forbidden_each_get_a_diagnostic() {
        let arena = Arena::new();
        let body = "## TODO\n\nx\n\n## OK\n\ny\n\n## Scratch\n\nz\n";
        let doc = doc_for(&arena, body);
        let rule = Rule {
            sections: vec!["TODO".into(), "Scratch".into()],
        };
        let mut diags = Vec::new();
        rule.check(&doc, &mut diags);
        assert_eq!(diags.len(), 2);
        let mut lines: Vec<_> = diags.iter().filter_map(|d| d.line).collect();
        lines.sort_unstable();
        assert_eq!(lines, vec![1, 9]);
    }

    #[test]
    fn h1_with_forbidden_text_does_not_match() {
        let arena = Arena::new();
        let body = "# TODO\n\nx\n";
        let doc = doc_for(&arena, body);
        let rule = Rule {
            sections: vec!["TODO".into()],
        };
        let mut diags = Vec::new();
        rule.check(&doc, &mut diags);
        assert!(
            diags.is_empty(),
            "H1 should not match the H2-only forbidden list: {diags:?}"
        );
    }

    #[test]
    fn body_line_offset_is_added() {
        let arena = Arena::new();
        let body = "## TODO\n";
        let ast = comrak::parse_document(&arena, body, &comrak::Options::default());
        let doc = ParsedDocument {
            path: PathBuf::from("fixture.md"),
            frontmatter: serde_json::Value::Null,
            ast,
            body_line_offset: 5,
        };
        let rule = Rule {
            sections: vec!["TODO".into()],
        };
        let mut diags = Vec::new();
        rule.check(&doc, &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].line, Some(5));
    }

    #[test]
    fn factory_parses_sections() {
        let factory = Factory;
        let params = json!({ "sections": ["TODO", "Scratch"] });
        let rule = factory.build(&params).expect("build");
        assert_eq!(rule.id(), ID);
    }

    #[test]
    fn factory_rejects_empty_sections() {
        let factory = Factory;
        let params = json!({ "sections": [] });
        assert!(factory.build(&params).is_err());
    }
}
