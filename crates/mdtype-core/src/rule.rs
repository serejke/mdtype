//! The `BodyRule` trait and factory interface.
//!
//! A rule examines a `ParsedDocument` and appends diagnostics. Rules are registered
//! via factories so schema loaders can construct them from YAML by rule id.

use crate::diagnostic::Diagnostic;
use crate::error::Error;
use crate::parser::ParsedDocument;

/// A body-structure check applied to a parsed document.
pub trait BodyRule: Send + Sync {
    /// Stable identifier, e.g. `"body.required_sections"`. Becomes the `rule` field on diagnostics.
    fn id(&self) -> &'static str;

    /// Run the rule against `doc` and append any findings to `out`.
    fn check(&self, doc: &ParsedDocument, out: &mut Vec<Diagnostic>);
}

/// Constructs a `BodyRule` from the YAML parameters attached to its entry in a schema file.
///
/// Schema loaders look up the factory for a rule id, pass the rule's YAML node, and receive
/// a fully configured rule instance.
pub trait BodyRuleFactory: Send + Sync {
    /// The rule id this factory produces. Matches `BodyRule::id`.
    fn id(&self) -> &'static str;

    /// Build a rule from its YAML parameters, or return `Err(Error::Schema(_))`.
    ///
    /// # Errors
    ///
    /// Returns `Error::Schema` if the params are missing required fields or have the wrong shape.
    fn build(&self, params: &serde_json::Value) -> Result<Box<dyn BodyRule>, Error>;
}
