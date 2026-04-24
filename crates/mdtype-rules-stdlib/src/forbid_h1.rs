//! `body.forbid_h1` — disallow any top-level `#` heading.

use mdtype_core::{BodyRule, BodyRuleFactory, Diagnostic, Error, ParsedDocument};

/// Rule id, exposed as a constant for downstream crates to reference.
pub const ID: &str = "body.forbid_h1";

/// The rule itself. Stateless — no configuration.
pub struct Rule;

impl BodyRule for Rule {
    fn id(&self) -> &'static str {
        ID
    }

    fn check(&self, _doc: &ParsedDocument, _out: &mut Vec<Diagnostic>) {
        todo!("implemented in Phase 3.2")
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
