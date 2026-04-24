//! `body.section_order` — assert ordering of required H2 headings.
//!
//! Two modes:
//! - `strict`: listed sections must appear in order with no other H2s between them
//!   (extras at the start or after the last required are still allowed).
//! - `relaxed`: listed sections must appear in order; other H2s may interleave anywhere.

use mdtype_core::nodes::{AstNode, NodeValue};
use mdtype_core::{BodyRule, BodyRuleFactory, Diagnostic, Error, Fixit, ParsedDocument, Severity};
use serde::Deserialize;

/// Rule id.
pub const ID: &str = "body.section_order";

/// Ordering enforcement mode.
#[derive(Debug, Clone, Copy)]
pub enum Mode {
    /// Required sections must appear in the given order with no other H2s between them.
    Strict,
    /// Required sections must appear in the given order; other H2s may interleave.
    Relaxed,
}

/// Configured rule instance.
pub struct Rule {
    /// Sections in their required order.
    pub order: Vec<String>,
    /// Enforcement mode.
    pub mode: Mode,
}

impl BodyRule for Rule {
    fn id(&self) -> &'static str {
        ID
    }

    fn check(&self, doc: &ParsedDocument, out: &mut Vec<Diagnostic>) {
        let h2s = collect_h2_headings(doc);
        let body_offset = doc.body_line_offset.saturating_sub(1);

        // Missing required
        for required in &self.order {
            let present = h2s.iter().any(|h| h.text == *required);
            if !present {
                out.push(Diagnostic {
                    file: doc.path.clone(),
                    line: None,
                    rule: ID,
                    severity: Severity::Error,
                    message: format!("missing required section '{required}'"),
                    fixit: Some(Fixit::AppendSection {
                        heading: format!("## {required}"),
                        after: None,
                    }),
                });
            }
        }

        // Walk the H2 sequence and tag each required heading with its position in `order`.
        let required_seq: Vec<RequiredHit> = h2s
            .iter()
            .enumerate()
            .filter_map(|(h2_idx, h)| {
                self.order
                    .iter()
                    .position(|s| s == &h.text)
                    .map(|order_idx| RequiredHit {
                        order_idx,
                        h2_idx,
                        line: h.body_line + body_offset,
                        text: h.text.clone(),
                    })
            })
            .collect();

        // Out-of-order: order_idx must be monotonically non-decreasing along the sequence.
        for window in required_seq.windows(2) {
            let prev = &window[0];
            let curr = &window[1];
            if curr.order_idx < prev.order_idx {
                out.push(Diagnostic {
                    file: doc.path.clone(),
                    line: Some(curr.line),
                    rule: ID,
                    severity: Severity::Error,
                    message: format!(
                        "section '{}' is out of order: should appear before '{}'",
                        curr.text, prev.text
                    ),
                    fixit: None,
                });
            }
        }

        // Strict: nothing other than the listed sections may appear between two consecutive
        // required hits.
        if matches!(self.mode, Mode::Strict) {
            for window in required_seq.windows(2) {
                let prev_h2 = window[0].h2_idx;
                let curr_h2 = window[1].h2_idx;
                for between in &h2s[prev_h2 + 1..curr_h2] {
                    out.push(Diagnostic {
                        file: doc.path.clone(),
                        line: Some(between.body_line + body_offset),
                        rule: ID,
                        severity: Severity::Error,
                        message: format!(
                            "unexpected section '{}' between required sections (strict mode)",
                            between.text
                        ),
                        fixit: None,
                    });
                }
            }
        }
    }
}

struct RequiredHit {
    order_idx: usize,
    h2_idx: usize,
    line: usize,
    text: String,
}

struct H2Heading {
    text: String,
    body_line: usize,
}

fn collect_h2_headings(doc: &ParsedDocument<'_>) -> Vec<H2Heading> {
    let mut out = Vec::new();
    for node in doc.ast.descendants() {
        let data = node.data.borrow();
        let NodeValue::Heading(h) = &data.value else {
            continue;
        };
        if h.level != 2 {
            continue;
        }
        out.push(H2Heading {
            text: heading_text(node),
            body_line: data.sourcepos.start.line,
        });
    }
    out
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

/// Factory. Params shape: `{ order: [String, ...], mode: "strict" | "relaxed" }`.
pub struct Factory;

impl BodyRuleFactory for Factory {
    fn id(&self) -> &'static str {
        ID
    }

    fn build(&self, params: &serde_json::Value) -> Result<Box<dyn BodyRule>, Error> {
        let parsed: Params = serde_json::from_value(params.clone())
            .map_err(|e| Error::Schema(format!("{ID}: invalid params: {e}")))?;
        if parsed.order.is_empty() {
            return Err(Error::Schema(format!("{ID}: `order` must not be empty")));
        }
        let mode = match parsed.mode {
            ModeYaml::Strict => Mode::Strict,
            ModeYaml::Relaxed => Mode::Relaxed,
        };
        Ok(Box::new(Rule {
            order: parsed.order,
            mode,
        }))
    }
}

#[derive(Debug, Deserialize)]
struct Params {
    order: Vec<String>,
    #[serde(default)]
    mode: ModeYaml,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ModeYaml {
    #[default]
    Relaxed,
    Strict,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use mdtype_core::{comrak, Arena, BodyRule, BodyRuleFactory, ParsedDocument};
    use serde_json::json;

    use super::{Factory, Mode, Rule, ID};

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

    fn order(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn correct_order_clean_in_relaxed() {
        let arena = Arena::new();
        let body = "## Summary\n\nx\n\n## Background\n\ny\n\n## Conclusion\n\nz\n";
        let doc = doc_for(&arena, body);
        let rule = Rule {
            order: order(&["Summary", "Background", "Conclusion"]),
            mode: Mode::Relaxed,
        };
        let mut diags = Vec::new();
        rule.check(&doc, &mut diags);
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn correct_order_clean_in_strict() {
        let arena = Arena::new();
        let body = "## Summary\n\nx\n\n## Background\n\ny\n\n## Conclusion\n\nz\n";
        let doc = doc_for(&arena, body);
        let rule = Rule {
            order: order(&["Summary", "Background", "Conclusion"]),
            mode: Mode::Strict,
        };
        let mut diags = Vec::new();
        rule.check(&doc, &mut diags);
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn inverted_order_relaxed_flags() {
        let arena = Arena::new();
        let body = "## Background\n\ny\n\n## Summary\n\nx\n";
        let doc = doc_for(&arena, body);
        let rule = Rule {
            order: order(&["Summary", "Background"]),
            mode: Mode::Relaxed,
        };
        let mut diags = Vec::new();
        rule.check(&doc, &mut diags);
        assert_eq!(diags.len(), 1);
        assert!(
            diags[0].message.contains("out of order"),
            "{}",
            diags[0].message
        );
        assert_eq!(diags[0].rule, ID);
    }

    #[test]
    fn inverted_order_strict_flags_too() {
        let arena = Arena::new();
        let body = "## Background\n\ny\n\n## Summary\n\nx\n";
        let doc = doc_for(&arena, body);
        let rule = Rule {
            order: order(&["Summary", "Background"]),
            mode: Mode::Strict,
        };
        let mut diags = Vec::new();
        rule.check(&doc, &mut diags);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("out of order"));
    }

    #[test]
    fn extra_section_between_relaxed_is_clean() {
        let arena = Arena::new();
        let body = "## Summary\n\nx\n\n## Aside\n\na\n\n## Background\n\ny\n";
        let doc = doc_for(&arena, body);
        let rule = Rule {
            order: order(&["Summary", "Background"]),
            mode: Mode::Relaxed,
        };
        let mut diags = Vec::new();
        rule.check(&doc, &mut diags);
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn extra_section_between_strict_flags() {
        let arena = Arena::new();
        let body = "## Summary\n\nx\n\n## Aside\n\na\n\n## Background\n\ny\n";
        let doc = doc_for(&arena, body);
        let rule = Rule {
            order: order(&["Summary", "Background"]),
            mode: Mode::Strict,
        };
        let mut diags = Vec::new();
        rule.check(&doc, &mut diags);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("Aside"), "{}", diags[0].message);
        assert!(diags[0].message.contains("strict"), "{}", diags[0].message);
        assert_eq!(diags[0].line, Some(5));
    }

    #[test]
    fn missing_required_section_flags_in_both_modes() {
        let arena = Arena::new();
        let body = "## Summary\n\nx\n";
        let doc = doc_for(&arena, body);

        for mode in [Mode::Relaxed, Mode::Strict] {
            let rule = Rule {
                order: order(&["Summary", "Background"]),
                mode,
            };
            let mut diags = Vec::new();
            rule.check(&doc, &mut diags);
            assert_eq!(diags.len(), 1, "mode={mode:?}");
            assert!(diags[0].message.contains("missing"), "{}", diags[0].message);
            assert!(diags[0].message.contains("Background"));
            assert!(diags[0].line.is_none());
        }
    }

    #[test]
    fn extras_at_start_or_end_clean_in_strict() {
        let arena = Arena::new();
        // Aside before Summary; Postscript after Background. Both allowed in strict.
        let body = "## Aside\n\na\n\n## Summary\n\nx\n\n## Background\n\ny\n\n## Postscript\n\np\n";
        let doc = doc_for(&arena, body);
        let rule = Rule {
            order: order(&["Summary", "Background"]),
            mode: Mode::Strict,
        };
        let mut diags = Vec::new();
        rule.check(&doc, &mut diags);
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn factory_defaults_to_relaxed() {
        let factory = Factory;
        let params = json!({ "order": ["A", "B"] });
        let rule = factory.build(&params).expect("build");
        assert_eq!(rule.id(), ID);
    }

    #[test]
    fn factory_parses_strict_mode() {
        let factory = Factory;
        let params = json!({ "order": ["A"], "mode": "strict" });
        assert!(factory.build(&params).is_ok());
    }

    #[test]
    fn factory_rejects_empty_order() {
        let factory = Factory;
        let params = json!({ "order": [] });
        assert!(factory.build(&params).is_err());
    }

    #[test]
    fn factory_rejects_unknown_mode() {
        let factory = Factory;
        let params = json!({ "order": ["A"], "mode": "loose" });
        assert!(factory.build(&params).is_err());
    }
}
