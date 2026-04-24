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

use mdtype_core::BodyRuleFactory;

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
