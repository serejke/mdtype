//! YAML-backed `SchemaSource`.
//!
//! Reads a `.mdtype.yaml` glob-map config, loads each referenced schema file, parses
//! frontmatter blocks as JSON Schema values and body blocks as lists of rule invocations
//! resolved via caller-supplied `BodyRuleFactory`s.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use mdtype_core::{BodyRuleFactory, Error, SchemaEntry, SchemaSource};

/// Walk upward from `start` looking for the nearest `.mdtype.yaml`.
///
/// Returns the first match or `None` if the filesystem root is reached without one.
#[must_use]
pub fn config_walk_up(_start: &Path) -> Option<PathBuf> {
    todo!("implemented in Phase 2.1")
}

/// YAML-backed source. Constructed with the path to the config file and the rule registry
/// it should use when instantiating body rules.
pub struct YamlSchemaSource {
    /// Path to `.mdtype.yaml`.
    pub config_path: PathBuf,
    /// Factories for every body-rule id that may appear in loaded schemas.
    pub factories: Arc<Vec<Box<dyn BodyRuleFactory>>>,
}

impl SchemaSource for YamlSchemaSource {
    fn load(&self) -> Result<Vec<SchemaEntry>, Error> {
        todo!("implemented in Phase 2.1")
    }
}
