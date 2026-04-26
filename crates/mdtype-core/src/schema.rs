//! In-memory schema representation. Two halves: frontmatter (JSON Schema) + body (rules).

use crate::rule::{BodyRule, WorkspaceRule};

/// One typed-reference spec produced by the schema loader from an `x-entity` annotation.
///
/// Pure data, shared between the loader (which produces specs) and the rule crate (which
/// consumes them). See `docs/types.md`.
#[derive(Debug, Clone)]
pub struct ReferenceSpec {
    /// Frontmatter key on the file under check.
    pub field: String,
    /// Expected entity names. Length 1 for a single target; length ≥ 2 for a union.
    pub targets: Vec<String>,
}

/// A fully loaded schema ready for validation.
#[derive(Default)]
pub struct Schema {
    /// Stable name, typically the schema file stem (e.g., `"blog-post"`).
    pub name: String,
    /// Human-readable description, surfaced in errors.
    pub description: Option<String>,
    /// Optional entity name for files attached to this schema. Other schemas' typed
    /// references may demand that fields point at files of a specific entity. Same-name
    /// entities across multiple schemas are allowed — the entity is a class, not a
    /// singleton. See `docs/types.md`.
    pub entity: Option<String>,
    /// JSON Schema document for the frontmatter, or `None` to skip frontmatter validation.
    pub frontmatter: Option<serde_json::Value>,
    /// Typed-reference specs synthesised by the schema loader from `x-entity` annotations
    /// in the frontmatter JSON Schema. Empty for schemas without typed references; non-empty
    /// schemas trigger an implicit `types.entity_ref` workspace rule installed by
    /// `mdtype-rules-stdlib::install_type_checks`.
    pub reference_specs: Vec<ReferenceSpec>,
    /// Ordered list of body rules. Rules execute in declaration order.
    pub body: Vec<Box<dyn BodyRule>>,
    /// Ordered list of workspace rules. Each entry is judged with the global
    /// [`crate::Workspace`] in scope; rules emit diagnostics only for files attached to
    /// this schema.
    pub workspace: Vec<Box<dyn WorkspaceRule>>,
}
