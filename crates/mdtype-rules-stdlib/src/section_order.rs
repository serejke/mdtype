//! `body.section_order` — assert ordering of required H2 headings.
//!
//! Two modes:
//! - `strict`: listed sections must appear in order with no other H2s between them.
//! - `relaxed`: listed sections must appear in order; other H2s may be interleaved.

use mdtype_core::{BodyRule, BodyRuleFactory, Diagnostic, Error, ParsedDocument};

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

    fn check(&self, _doc: &ParsedDocument, _out: &mut Vec<Diagnostic>) {
        todo!("implemented in Phase 3.4")
    }
}

/// Factory. Params shape: `{ order: [String, ...], mode: "strict" | "relaxed" }`.
pub struct Factory;

impl BodyRuleFactory for Factory {
    fn id(&self) -> &'static str {
        ID
    }

    fn build(&self, _params: &serde_json::Value) -> Result<Box<dyn BodyRule>, Error> {
        todo!("implemented in Phase 3.4")
    }
}
