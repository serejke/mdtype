//! `body.required_sections` — assert that each named H2 heading exists.

use mdtype_core::{BodyRule, BodyRuleFactory, Diagnostic, Error, ParsedDocument};

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

    fn check(&self, _doc: &ParsedDocument, _out: &mut Vec<Diagnostic>) {
        todo!("implemented in Phase 3.3")
    }
}

/// Factory. Params shape: `{ sections: [String, ...] }`.
pub struct Factory;

impl BodyRuleFactory for Factory {
    fn id(&self) -> &'static str {
        ID
    }

    fn build(&self, _params: &serde_json::Value) -> Result<Box<dyn BodyRule>, Error> {
        todo!("implemented in Phase 3.3")
    }
}
