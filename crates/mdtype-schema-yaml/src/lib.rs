//! YAML-backed `SchemaSource`.
//!
//! Reads a `.mdtype.yaml` glob-map config, loads each referenced schema file, parses
//! frontmatter blocks as JSON Schema values and body blocks as lists of rule invocations
//! resolved via caller-supplied `BodyRuleFactory`s.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use mdtype_core::{BodyRule, BodyRuleFactory, Error, Schema, SchemaEntry, SchemaSource};
use serde::Deserialize;

/// File name of the glob-map config that `mdtype` discovers by walking up from the cwd.
pub const CONFIG_FILE_NAME: &str = ".mdtype.yaml";

/// YAML-backed source.
///
/// Constructed with the path to `.mdtype.yaml` and a registry of body-rule factories.
/// Each `body:` entry in a referenced schema file is resolved against this registry; an
/// unknown rule id surfaces as [`Error::Schema`] (CLI exit `2`).
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
            let schema = load_schema_file(&schema_path, &self.factories)?;
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

/// Load a single schema YAML file off disk and parse it into [`Schema`].
///
/// Each `body:` entry is resolved against `factories`. Both the canonical factory id and the
/// kebab-case shortform (canonical id with `body.` stripped and `_` rewritten to `-`) are
/// accepted, so YAML can stay idiomatic while diagnostics carry the canonical id.
///
/// # Errors
///
/// Returns [`Error::Io`] if `path` cannot be read and [`Error::Schema`] if the YAML is
/// malformed, the frontmatter section cannot be converted to JSON, an entry omits its `rule`
/// key, or a referenced rule id has no matching factory.
pub fn load_schema_file(
    path: &Path,
    factories: &[Box<dyn BodyRuleFactory>],
) -> Result<Schema, Error> {
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

    let lookup = build_factory_lookup(factories);
    let mut body: Vec<Box<dyn BodyRule>> = Vec::with_capacity(parsed.body.len());
    for (idx, entry) in parsed.body.into_iter().enumerate() {
        body.push(build_body_rule(path, idx, entry, factories, &lookup)?);
    }

    Ok(Schema {
        name: parsed.name,
        description: parsed.description,
        frontmatter,
        body,
    })
}

fn build_body_rule(
    schema_path: &Path,
    idx: usize,
    entry: serde_yaml::Value,
    factories: &[Box<dyn BodyRuleFactory>],
    lookup: &HashMap<String, usize>,
) -> Result<Box<dyn BodyRule>, Error> {
    let serde_yaml::Value::Mapping(mut map) = entry else {
        return Err(Error::Schema(format!(
            "body[{idx}] in {} must be a mapping",
            schema_path.display()
        )));
    };

    let rule_id_value = map.remove("rule").ok_or_else(|| {
        Error::Schema(format!(
            "body[{idx}] in {} is missing the required `rule` key",
            schema_path.display()
        ))
    })?;
    let rule_id = rule_id_value.as_str().ok_or_else(|| {
        Error::Schema(format!(
            "body[{idx}] in {}: `rule` must be a string",
            schema_path.display()
        ))
    })?;

    let factory_idx = lookup.get(rule_id).copied().ok_or_else(|| {
        Error::Schema(format!(
            "unknown body-rule id `{rule_id}` in body[{idx}] of {}",
            schema_path.display()
        ))
    })?;

    let params_yaml = serde_yaml::Value::Mapping(map);
    let params_json = serde_json::to_value(&params_yaml).map_err(|e| {
        Error::Schema(format!(
            "body[{idx}] params yaml→json in {}: {e}",
            schema_path.display()
        ))
    })?;

    factories[factory_idx].build(&params_json)
}

fn build_factory_lookup(factories: &[Box<dyn BodyRuleFactory>]) -> HashMap<String, usize> {
    let mut map: HashMap<String, usize> = HashMap::new();
    for (i, factory) in factories.iter().enumerate() {
        let id = factory.id();
        map.insert(id.to_string(), i);
        if let Some(short) = id.strip_prefix("body.") {
            let kebab = short.replace('_', "-");
            map.entry(kebab).or_insert(i);
        }
    }
    map
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
    #[serde(default)]
    body: Vec<serde_yaml::Value>,
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::Arc;

    use mdtype_core::{
        BodyRule, BodyRuleFactory, Diagnostic, Error, ParsedDocument, SchemaSource,
    };
    use tempfile::tempdir;

    use super::{config_walk_up, load_schema_file, YamlSchemaSource, CONFIG_FILE_NAME};

    struct DummyRule;
    impl BodyRule for DummyRule {
        fn id(&self) -> &'static str {
            "body.forbid_h1"
        }
        fn check(&self, _doc: &ParsedDocument, _out: &mut Vec<Diagnostic>) {}
    }

    struct DummyFactory;
    impl BodyRuleFactory for DummyFactory {
        fn id(&self) -> &'static str {
            "body.forbid_h1"
        }
        fn build(&self, _params: &serde_json::Value) -> Result<Box<dyn BodyRule>, Error> {
            Ok(Box::new(DummyRule))
        }
    }

    fn dummy_registry() -> Arc<Vec<Box<dyn BodyRuleFactory>>> {
        Arc::new(vec![Box::new(DummyFactory) as Box<dyn BodyRuleFactory>])
    }

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

        let src =
            YamlSchemaSource::new(dir.path().join(CONFIG_FILE_NAME), Arc::new(Vec::new()));
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
    fn body_rule_resolves_via_kebab_alias() {
        let dir = tempdir().expect("tempdir");
        let schema_path = dir.path().join("blog-post.yaml");
        fs::write(
            &schema_path,
            "name: blog-post\nfrontmatter: { type: object }\nbody:\n  - rule: forbid-h1\n",
        )
        .unwrap();

        let factories = dummy_registry();
        let schema = load_schema_file(&schema_path, &factories).expect("load");
        assert_eq!(schema.body.len(), 1);
        assert_eq!(schema.body[0].id(), "body.forbid_h1");
    }

    #[test]
    fn body_rule_resolves_via_canonical_id() {
        let dir = tempdir().expect("tempdir");
        let schema_path = dir.path().join("blog-post.yaml");
        fs::write(
            &schema_path,
            "name: blog-post\nbody:\n  - rule: body.forbid_h1\n",
        )
        .unwrap();

        let factories = dummy_registry();
        let schema = load_schema_file(&schema_path, &factories).expect("load");
        assert_eq!(schema.body.len(), 1);
    }

    #[test]
    fn unknown_body_rule_is_a_schema_error() {
        let dir = tempdir().expect("tempdir");
        let schema_path = dir.path().join("blog-post.yaml");
        fs::write(
            &schema_path,
            "name: blog-post\nbody:\n  - rule: not-a-real-rule\n",
        )
        .unwrap();

        let factories = dummy_registry();
        let result = load_schema_file(&schema_path, &factories);
        match result {
            Err(Error::Schema(msg)) => {
                assert!(
                    msg.contains("not-a-real-rule"),
                    "expected error to mention the rule id, got: {msg}"
                );
            }
            Err(other) => panic!("expected Error::Schema, got {other}"),
            Ok(_) => panic!("expected error, got Ok"),
        }
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
