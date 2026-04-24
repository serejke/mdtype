//! `body.forbidden_sections` — assert that named H2 headings do not appear.

use mdtype_core::{BodyRule, BodyRuleFactory, Diagnostic, Error, ParsedDocument};

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

    fn check(&self, _doc: &ParsedDocument, _out: &mut Vec<Diagnostic>) {
        todo!("implemented in Phase 3.5")
    }
}

/// Factory. Params shape: `{ sections: [String, ...] }`.
pub struct Factory;

impl BodyRuleFactory for Factory {
    fn id(&self) -> &'static str {
        ID
    }

    fn build(&self, _params: &serde_json::Value) -> Result<Box<dyn BodyRule>, Error> {
        todo!("implemented in Phase 3.5")
    }
}
