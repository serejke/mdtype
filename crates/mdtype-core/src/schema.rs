//! In-memory schema representation. Two halves: frontmatter (JSON Schema) + body (rules).

use crate::rule::{BodyRule, WorkspaceRule};

/// A fully loaded schema ready for validation.
#[derive(Default)]
pub struct Schema {
    /// Stable name, typically the schema file stem (e.g., `"blog-post"`).
    pub name: String,
    /// Human-readable description, surfaced in errors.
    pub description: Option<String>,
    /// JSON Schema document for the frontmatter, or `None` to skip frontmatter validation.
    pub frontmatter: Option<serde_json::Value>,
    /// Ordered list of body rules. Rules execute in declaration order.
    pub body: Vec<Box<dyn BodyRule>>,
    /// Ordered list of workspace rules. Each entry is judged with the global
    /// [`crate::Workspace`] in scope; rules emit diagnostics only for files attached to
    /// this schema.
    pub workspace: Vec<Box<dyn WorkspaceRule>>,
}
