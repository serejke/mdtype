//! In-memory schema representation. Two halves: frontmatter (JSON Schema) + body (rules).

use crate::rule::BodyRule;

/// A fully loaded schema ready for validation.
pub struct Schema {
    /// Stable name, typically the schema file stem (e.g., `"blog-post"`).
    pub name: String,
    /// Human-readable description, surfaced in errors.
    pub description: Option<String>,
    /// JSON Schema document for the frontmatter, or `None` to skip frontmatter validation.
    pub frontmatter: Option<serde_json::Value>,
    /// Ordered list of body rules. Rules execute in declaration order.
    pub body: Vec<Box<dyn BodyRule>>,
}
