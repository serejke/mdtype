//! The `SchemaSource` trait — any backing store that can produce `(glob, Schema)` pairs.

use crate::error::Error;
use crate::schema::Schema;

/// A single glob-to-schema binding returned from a source.
pub struct SchemaEntry {
    /// Glob pattern relative to the config root, e.g. `"content/posts/**/*.md"`.
    pub glob: String,
    /// Fully loaded schema that files matching `glob` must conform to.
    pub schema: Schema,
}

/// Produce schema bindings from some backing store (YAML on disk, JSON, HTTP, tests, …).
pub trait SchemaSource {
    /// Load and return all known bindings.
    ///
    /// # Errors
    ///
    /// Returns `Error::Schema` on malformed config or unknown rule ids.
    fn load(&self) -> Result<Vec<SchemaEntry>, Error>;
}
