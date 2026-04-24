//! YAML-backed `SchemaSource`.
//!
//! Reads a `.mdtype.yaml` glob-map config, loads each referenced schema file, parses
//! frontmatter blocks as JSON Schema values and body blocks as lists of rule invocations
//! resolved via caller-supplied `BodyRuleFactory`s.

#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use mdtype_core::{BodyRuleFactory, Error, Schema, SchemaEntry, SchemaSource};
use serde::Deserialize;

/// File name of the glob-map config that `mdtype` discovers by walking up from the cwd.
pub const CONFIG_FILE_NAME: &str = ".mdtype.yaml";

/// YAML-backed source.
///
/// Constructed with the path to `.mdtype.yaml` and a registry of body-rule factories.
/// The registry is unused while loading until Phase 3.1 wires body rules through it; it is
/// stored now so callers do not have to thread it through later.
pub struct YamlSchemaSource {
    /// Path to `.mdtype.yaml`.
    pub config_path: PathBuf,
    /// Directory containing the config; relative `schema:` paths resolve against this root.
    pub root: PathBuf,
    /// Factories for every body-rule id that may appear in loaded schemas.
    pub factories: Arc<Vec<Box<dyn BodyRuleFactory>>>,
}

impl YamlSchemaSource {
    /// Build a source from a config path and a (possibly empty) factory registry.
    ///
    /// The config's parent directory becomes [`root`](Self::root). Falls back to `"."` when
    /// `config_path` has no parent component.
    #[must_use]
    pub fn new(config_path: PathBuf, factories: Arc<Vec<Box<dyn BodyRuleFactory>>>) -> Self {
        let root = config_path
            .parent()
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
        Self {
            config_path,
            root,
            factories,
        }
    }
}

impl SchemaSource for YamlSchemaSource {
    fn load(&self) -> Result<Vec<SchemaEntry>, Error> {
        let raw = fs::read_to_string(&self.config_path).map_err(|source| Error::Io {
            path: self.config_path.clone(),
            source,
        })?;
        let cfg: ConfigFile = serde_yaml::from_str(&raw).map_err(|e| {
            Error::Schema(format!(
                "malformed config {}: {e}",
                self.config_path.display()
            ))
        })?;

        let mut out = Vec::with_capacity(cfg.rules.len());
        for entry in cfg.rules {
            let schema_path = if entry.schema.is_absolute() {
                entry.schema.clone()
            } else {
                self.root.join(&entry.schema)
            };
            let schema = load_schema_file(&schema_path)?;
            out.push(SchemaEntry {
                glob: entry.glob,
                schema,
            });
        }
        Ok(out)
    }
}

/// Walk upward from `start` looking for the nearest [`CONFIG_FILE_NAME`].
///
/// `start` may be a file or a directory. Returns the first match or `None` if the filesystem
/// root is reached without one.
#[must_use]
pub fn config_walk_up(start: &Path) -> Option<PathBuf> {
    let mut current = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };
    loop {
        let candidate = current.join(CONFIG_FILE_NAME);
        if candidate.is_file() {
            return Some(candidate);
        }
        if !current.pop() {
            return None;
        }
    }
}

fn load_schema_file(path: &Path) -> Result<Schema, Error> {
    let raw = fs::read_to_string(path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let parsed: SchemaFile = serde_yaml::from_str(&raw)
        .map_err(|e| Error::Schema(format!("malformed schema {}: {e}", path.display())))?;
    let frontmatter = match parsed.frontmatter {
        Some(value) => Some(serde_json::to_value(value).map_err(|e| {
            Error::Schema(format!(
                "frontmatter→json conversion in {}: {e}",
                path.display()
            ))
        })?),
        None => None,
    };
    Ok(Schema {
        name: parsed.name,
        description: parsed.description,
        frontmatter,
        // Body rules are resolved against the factory registry in Phase 3.1.
        body: Vec::new(),
    })
}

#[derive(Debug, Deserialize)]
struct ConfigFile {
    rules: Vec<RuleEntry>,
}

#[derive(Debug, Deserialize)]
struct RuleEntry {
    glob: String,
    schema: PathBuf,
}

#[derive(Debug, Deserialize)]
struct SchemaFile {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    frontmatter: Option<serde_yaml::Value>,
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::Arc;

    use mdtype_core::SchemaSource;
    use tempfile::tempdir;

    use super::{config_walk_up, YamlSchemaSource, CONFIG_FILE_NAME};

    #[test]
    fn loads_a_config_with_one_schema_entry() {
        let dir = tempdir().expect("tempdir");
        let schemas_dir = dir.path().join("schemas");
        fs::create_dir_all(&schemas_dir).unwrap();

        fs::write(
            dir.path().join(CONFIG_FILE_NAME),
            "rules:\n  - glob: \"posts/**/*.md\"\n    schema: schemas/blog-post.yaml\n",
        )
        .unwrap();
        fs::write(
            schemas_dir.join("blog-post.yaml"),
            "name: blog-post\ndescription: A post.\nfrontmatter:\n  type: object\n  required: [title]\n  properties:\n    title: { type: string }\nbody: []\n",
        )
        .unwrap();

        let src = YamlSchemaSource::new(dir.path().join(CONFIG_FILE_NAME), Arc::new(Vec::new()));
        let entries = src.load().expect("load");

        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        assert_eq!(entry.glob, "posts/**/*.md");
        assert_eq!(entry.schema.name, "blog-post");
        assert_eq!(entry.schema.description.as_deref(), Some("A post."));
        let fm = entry.schema.frontmatter.as_ref().expect("frontmatter present");
        assert_eq!(fm["type"], serde_json::json!("object"));
        assert_eq!(fm["required"], serde_json::json!(["title"]));
        assert!(entry.schema.body.is_empty());
    }

    #[test]
    fn config_walk_up_finds_nearest_config() {
        let dir = tempdir().expect("tempdir");
        let nested = dir.path().join("a/b/c");
        fs::create_dir_all(&nested).unwrap();
        let cfg = dir.path().join(CONFIG_FILE_NAME);
        fs::write(&cfg, "rules: []\n").unwrap();

        let found = config_walk_up(&nested).expect("found");
        assert_eq!(
            fs::canonicalize(&found).unwrap(),
            fs::canonicalize(&cfg).unwrap()
        );
    }

    #[test]
    fn config_walk_up_returns_none_when_absent() {
        let dir = tempdir().expect("tempdir");
        assert!(config_walk_up(dir.path()).is_none());
    }
}
