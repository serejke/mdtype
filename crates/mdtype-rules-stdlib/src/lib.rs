//! Built-in body rules for `mdtype`.
//!
//! Each rule lives in its own module and is trivially copy-pasteable as a template for
//! new rules in downstream crates. Add new rules here **or** in an external crate — never
//! in `mdtype-core`.

#![forbid(unsafe_code)]

pub mod forbid_h1;
pub mod forbidden_sections;
pub mod required_sections;
pub mod section_order;

use mdtype_core::nodes::{AstNode, NodeValue};
use mdtype_core::BodyRuleFactory;

/// Concatenate the rendered text of a heading node's children.
///
/// Strong/emphasis spans are flattened, fenced code spans contribute their literal,
/// all other inlines are ignored. Used by every section-matching rule so the rules
/// agree on what `## *Summary*` "is".
pub(crate) fn heading_text<'a>(heading: &'a AstNode<'a>) -> String {
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

/// Return the set of factories for every stdlib rule. Register these with your
/// `SchemaSource` so YAML schemas may reference their rule ids.
#[must_use]
pub fn register_stdlib() -> Vec<Box<dyn BodyRuleFactory>> {
    vec![
        Box::new(forbid_h1::Factory),
        Box::new(required_sections::Factory),
        Box::new(section_order::Factory),
        Box::new(forbidden_sections::Factory),
    ]
}
