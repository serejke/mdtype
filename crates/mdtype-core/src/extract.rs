//! Fact extractors used by [`crate::run_workspace`].
//!
//! Each extractor walks a [`ParsedDocument`]'s comrak AST and writes the facts the
//! corresponding [`crate::Requirements`] flag asks for. Extractors are pure: they never
//! emit diagnostics and never decide policy.

use std::collections::HashMap;

use comrak::nodes::{AstNode, NodeValue};

use crate::parser::ParsedDocument;
use crate::workspace::{HeadingFact, LinkKind, LinkRef, Requirements};

/// Walk `doc` and append every heading occurrence to `out`.
///
/// Slugs follow GitHub's algorithm: lowercase Unicode-aware, whitespace and underscores
/// collapsed to `-`, non-alphanumeric punctuation dropped, runs of `-` merged. Repeated
/// headings within the same file get a `-1`, `-2`, … suffix in source order so a link
/// can disambiguate (`#intro`, `#intro-1`, `#intro-2`).
pub fn extract_headings(doc: &ParsedDocument<'_>, out: &mut Vec<HeadingFact>) {
    let mut seen: HashMap<String, u32> = HashMap::new();
    for node in doc.ast.descendants() {
        let data = node.data.borrow();
        let NodeValue::Heading(h) = &data.value else {
            continue;
        };
        let text = node_text(node);
        let base = slugify(&text);
        let occurrence = seen.entry(base.clone()).or_insert(0);
        let slug = if *occurrence == 0 {
            base.clone()
        } else {
            format!("{base}-{occurrence}")
        };
        *occurrence += 1;
        let line = data.sourcepos.start.line + doc.body_line_offset.saturating_sub(1);
        out.push(HeadingFact {
            text,
            slug,
            level: h.level,
            line,
        });
    }
}

/// Walk `doc` and append every link occurrence whose kind is enabled by `reqs`.
///
/// `reqs.links_inline` controls `Link` (and post-resolution reference) nodes.
/// `reqs.links_wiki` controls `WikiLink` nodes; the runner is responsible for enabling
/// comrak's wikilink extension when this flag is set, otherwise `[[ … ]]` never reaches
/// the AST as a wikilink.
pub fn extract_links(doc: &ParsedDocument<'_>, reqs: Requirements, out: &mut Vec<LinkRef>) {
    for node in doc.ast.descendants() {
        let data = node.data.borrow();
        let absolute_line = data.sourcepos.start.line + doc.body_line_offset.saturating_sub(1);
        match &data.value {
            NodeValue::Link(link) if reqs.links_inline => {
                let (target, anchor) = split_anchor(&link.url);
                out.push(LinkRef {
                    kind: LinkKind::Inline,
                    target,
                    anchor,
                    alias: None,
                    line: absolute_line,
                });
            }
            NodeValue::WikiLink(wl) if reqs.links_wiki => {
                let (target, anchor) = split_anchor(&wl.url);
                let rendered = node_text(node);
                let alias = if rendered.is_empty() || rendered == wl.url {
                    None
                } else {
                    Some(rendered)
                };
                out.push(LinkRef {
                    kind: LinkKind::Wiki,
                    target,
                    anchor,
                    alias,
                    line: absolute_line,
                });
            }
            _ => {}
        }
    }
}

fn split_anchor(url: &str) -> (String, Option<String>) {
    url.split_once('#').map_or_else(
        || (url.to_string(), None),
        |(p, a)| (p.to_string(), Some(a.to_string())),
    )
}

/// GitHub-flavored slug. Used for `[link](file.md#anchor)` resolution; Obsidian-style
/// `[[Note#Heading]]` resolvers should match against [`HeadingFact::text`] instead.
///
/// Algorithm (mirrors github.com's anchor generator):
///
/// 1. Lowercase, Unicode-aware (so `Café` keeps its `é`).
/// 2. Keep alphanumeric characters; map whitespace, `-`, and `_` to `-`; drop everything
///    else (punctuation, symbols).
/// 3. Collapse runs of `-` and trim from both ends.
fn slugify(text: &str) -> String {
    let mut buf = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_alphanumeric() {
            for lc in c.to_lowercase() {
                buf.push(lc);
            }
        } else if c.is_whitespace() || c == '-' || c == '_' {
            buf.push('-');
        }
    }
    buf.split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn node_text<'a>(node: &'a AstNode<'a>) -> String {
    let mut buf = String::new();
    for desc in node.descendants().skip(1) {
        let data = desc.data.borrow();
        match &data.value {
            NodeValue::Text(t) => buf.push_str(t),
            NodeValue::Code(c) => buf.push_str(&c.literal),
            _ => {}
        }
    }
    buf
}

#[cfg(test)]
mod tests {
    use comrak::Arena;
    use std::path::PathBuf;

    use super::{extract_headings, slugify, split_anchor};
    use crate::parser::ParsedDocument;

    #[test]
    fn slugify_handles_typical_headings() {
        assert_eq!(slugify("Setup & Teardown"), "setup-teardown");
        assert_eq!(slugify("  Multiple   spaces  "), "multiple-spaces");
        assert_eq!(slugify("punctuation, please!"), "punctuation-please");
    }

    #[test]
    fn slugify_keeps_unicode() {
        // Mirrors GitHub: `Café` → `café`, not `caf` (the previous algorithm dropped
        // non-ASCII alphanumerics, producing false-positive missing-anchor diagnostics).
        assert_eq!(slugify("UNICODE café"), "unicode-café");
        assert_eq!(slugify("日本語"), "日本語");
    }

    #[test]
    fn duplicate_headings_get_numeric_suffix() {
        let arena = Arena::new();
        let body = "## Intro\n\nfirst.\n\n## Intro\n\nsecond.\n\n## Intro\n\nthird.\n";
        let ast = comrak::parse_document(&arena, body, &comrak::Options::default());
        let doc = ParsedDocument {
            path: PathBuf::from("fixture.md"),
            frontmatter: serde_json::Value::Null,
            ast,
            body_line_offset: 1,
        };
        let mut headings = Vec::new();
        extract_headings(&doc, &mut headings);
        let slugs: Vec<&str> = headings.iter().map(|h| h.slug.as_str()).collect();
        assert_eq!(slugs, ["intro", "intro-1", "intro-2"]);
    }

    #[test]
    fn split_anchor_separates_fragment() {
        assert_eq!(
            split_anchor("file.md#section"),
            ("file.md".into(), Some("section".into()))
        );
        assert_eq!(split_anchor("file.md"), ("file.md".into(), None));
        assert_eq!(
            split_anchor("#anchor-only"),
            (String::new(), Some("anchor-only".into()))
        );
    }
}
