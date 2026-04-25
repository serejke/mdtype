//! `body.required_sections` — assert that each named H2 heading exists.

use std::collections::HashSet;

use mdtype_core::nodes::{AstNode, NodeValue};
use mdtype_core::{BodyRule, BodyRuleFactory, Diagnostic, Error, Fixit, ParsedDocument, Severity};
use serde::Deserialize;

use crate::heading_text;

/// Rule id.
pub const ID: &str = "body.required_sections";

/// Configured rule instance.
pub struct Rule {
    /// Exact heading texts (without `##`) that must appear as H2 headings.
    pub sections: Vec<String>,
}

impl BodyRule for Rule {
    fn id(&self) -> &'static str {
        ID
    }

    fn check(&self, doc: &ParsedDocument, out: &mut Vec<Diagnostic>) {
        let present = collect_h2_headings(doc.ast);
        for required in &self.sections {
            if !present.contains(required.as_str()) {
                out.push(Diagnostic {
                    file: doc.path.clone(),
                    line: None,
                    rule: ID,
                    severity: Severity::Error,
                    message: format!("missing H2 section '{required}'; add it as '## {required}'"),
                    fixit: Some(Fixit::AppendSection {
                        heading: format!("## {required}"),
                        after: None,
                    }),
                });
            }
        }
    }
}

fn collect_h2_headings<'a>(root: &'a AstNode<'a>) -> HashSet<String> {
    let mut out = HashSet::new();
    for node in root.descendants() {
        let data = node.data.borrow();
        let NodeValue::Heading(heading) = &data.value else {
            continue;
        };
        if heading.level != 2 {
            continue;
        }
        out.insert(heading_text(node));
    }
    out
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
    fn all_sections_present_emits_nothing() {
        let arena = Arena::new();
        let body = "## Summary\n\nx\n\n## Background\n\ny\n\n## Conclusion\n\nz\n";
        let doc = doc_for(&arena, body);
        let rule = Rule {
            sections: vec!["Summary".into(), "Background".into(), "Conclusion".into()],
        };
        let mut diags = Vec::new();
        rule.check(&doc, &mut diags);
        assert!(diags.is_empty(), "expected clean, got {diags:?}");
    }

    #[test]
    fn one_missing_emits_one_diagnostic() {
        let arena = Arena::new();
        let body = "## Summary\n\nx\n\n## Conclusion\n\nz\n";
        let doc = doc_for(&arena, body);
        let rule = Rule {
            sections: vec!["Summary".into(), "Background".into(), "Conclusion".into()],
        };
        let mut diags = Vec::new();
        rule.check(&doc, &mut diags);
        assert_eq!(diags.len(), 1);
        let d = &diags[0];
        assert_eq!(d.rule, ID);
        assert!(d.line.is_none());
        assert!(d.message.contains("Background"), "{}", d.message);
    }

    #[test]
    fn none_present_emits_one_diagnostic_per_required() {
        let arena = Arena::new();
        let body = "Just some prose with no headings at all.\n";
        let doc = doc_for(&arena, body);
        let rule = Rule {
            sections: vec!["Summary".into(), "Background".into()],
        };
        let mut diags = Vec::new();
        rule.check(&doc, &mut diags);
        assert_eq!(diags.len(), 2);
    }

    #[test]
    fn h2_with_inline_emphasis_matches_plain_text() {
        let arena = Arena::new();
        let body = "## *Summary*\n\nbody\n";
        let doc = doc_for(&arena, body);
        let rule = Rule {
            sections: vec!["Summary".into()],
        };
        let mut diags = Vec::new();
        rule.check(&doc, &mut diags);
        assert!(
            diags.is_empty(),
            "expected emphasis stripped, got {diags:?}"
        );
    }

    #[test]
    fn h1_does_not_satisfy_required_h2() {
        let arena = Arena::new();
        let body = "# Summary\n\nbody\n";
        let doc = doc_for(&arena, body);
        let rule = Rule {
            sections: vec!["Summary".into()],
        };
        let mut diags = Vec::new();
        rule.check(&doc, &mut diags);
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn factory_parses_sections() {
        let factory = Factory;
        let params = json!({ "sections": ["Summary", "Conclusion"] });
        let rule = factory.build(&params).expect("build");
        assert_eq!(rule.id(), ID);
    }

    #[test]
    fn factory_rejects_empty_sections() {
        let factory = Factory;
        let params = json!({ "sections": [] });
        assert!(factory.build(&params).is_err());
    }

    #[test]
    fn factory_rejects_missing_sections_key() {
        let factory = Factory;
        let params = json!({});
        assert!(factory.build(&params).is_err());
    }
}
