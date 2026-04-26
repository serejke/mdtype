//! YAML-backed `SchemaSource`.
//!
//! Reads a `.mdtype.yaml` glob-map config, loads each referenced schema file, parses
//! frontmatter blocks as JSON Schema values and `body:` / `links:` blocks as lists of
//! rule invocations resolved via caller-supplied factory registries.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use mdtype_core::{
    BodyRule, BodyRuleFactory, Error, ReferenceSpec, Schema, SchemaEntry, SchemaSource,
    WorkspaceRule, WorkspaceRuleFactory,
};
use serde::Deserialize;

mod entity_walker;

/// File name of the glob-map config that `mdtype` discovers by walking up from the cwd.
pub const CONFIG_FILE_NAME: &str = ".mdtype.yaml";

/// YAML-backed source.
///
/// Constructed with the path to `.mdtype.yaml` and two factory registries. Each `body:`
/// entry in a referenced schema file is resolved against `body_factories` and each
/// `links:` entry against `workspace_factories` filtered to ids prefixed with `links.`;
/// an unknown rule id surfaces as [`Error::Schema`] (CLI exit `2`).
pub struct YamlSchemaSource {
    /// Path to `.mdtype.yaml`.
    pub config_path: PathBuf,
    /// Directory containing the config; relative `schema:` paths resolve against this root.
    pub root: PathBuf,
    /// Factories for every body-rule id that may appear in loaded schemas.
    pub body_factories: Arc<Vec<Box<dyn BodyRuleFactory>>>,
    /// Factories for every workspace-rule id that may appear in loaded schemas.
    pub workspace_factories: Arc<Vec<Box<dyn WorkspaceRuleFactory>>>,
}

impl YamlSchemaSource {
    /// Build a source from a config path and two (possibly empty) factory registries.
    ///
    /// The config's parent directory becomes [`root`](Self::root). Falls back to `"."` when
    /// `config_path` has no parent component.
    #[must_use]
    pub fn new(
        config_path: PathBuf,
        body_factories: Arc<Vec<Box<dyn BodyRuleFactory>>>,
        workspace_factories: Arc<Vec<Box<dyn WorkspaceRuleFactory>>>,
    ) -> Self {
        let root = config_path
            .parent()
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
        Self {
            config_path,
            root,
            body_factories,
            workspace_factories,
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
            let schema = load_schema_file(
                &schema_path,
                &self.body_factories,
                &self.workspace_factories,
            )?;
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
/// Each `body:` entry is resolved against `body_factories`; each `links:` entry against
/// `workspace_factories` filtered to ids prefixed with `links.`. Both blocks accept
/// the canonical factory id and the kebab-case shortform (canonical id with the
/// `body.` or `links.` prefix stripped and `_` rewritten to `-`). The diagnostic `rule`
/// field carries the canonical form regardless of how the YAML referenced it.
///
/// The legacy `workspace:` block was removed; a schema that still declares one fails
/// to load with a precise migration hint.
///
/// # Errors
///
/// Returns [`Error::Io`] if `path` cannot be read and [`Error::Schema`] if the YAML is
/// malformed, the frontmatter section cannot be converted to JSON, an entry omits its
/// `rule` key, a referenced rule id has no matching factory in either registry, or the
/// schema declares the legacy `workspace:` block.
pub fn load_schema_file(
    path: &Path,
    body_factories: &[Box<dyn BodyRuleFactory>],
    workspace_factories: &[Box<dyn WorkspaceRuleFactory>],
) -> Result<Schema, Error> {
    let raw = fs::read_to_string(path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let parsed: SchemaFile = serde_yaml::from_str(&raw)
        .map_err(|e| Error::Schema(format!("malformed schema {}: {e}", path.display())))?;

    let entity = match parsed.entity {
        None => None,
        Some(name) if name.is_empty() => {
            return Err(Error::Schema(format!(
                "schema {}: `entity` must be a non-empty string",
                path.display()
            )));
        }
        Some(name) => Some(name),
    };

    let frontmatter = match parsed.frontmatter {
        Some(value) => Some(serde_json::to_value(value).map_err(|e| {
            Error::Schema(format!(
                "frontmatter→json conversion in {}: {e}",
                path.display()
            ))
        })?),
        None => None,
    };

    let reference_specs: Vec<ReferenceSpec> = match frontmatter.as_ref() {
        Some(fm) => entity_walker::walk(fm, path)?,
        None => Vec::new(),
    };

    let body_lookup = build_body_lookup(body_factories);
    let mut body: Vec<Box<dyn BodyRule>> = Vec::with_capacity(parsed.body.len());
    for (idx, entry) in parsed.body.into_iter().enumerate() {
        body.push(build_body_rule(
            path,
            idx,
            entry,
            body_factories,
            &body_lookup,
        )?);
    }

    if parsed.workspace.is_some() {
        return Err(Error::Schema(format!(
            "schema {}: the `workspace:` block was removed; move link rules under `links:` instead (and drop the `links.` prefix on rule ids — `links: [- rule: relative_path]`)",
            path.display()
        )));
    }

    let links_lookup = build_links_lookup(workspace_factories);
    let mut workspace: Vec<Box<dyn WorkspaceRule>> = Vec::with_capacity(parsed.links.len());
    for (idx, entry) in parsed.links.into_iter().enumerate() {
        workspace.push(build_links_rule(
            path,
            idx,
            entry,
            workspace_factories,
            &links_lookup,
        )?);
    }

    Ok(Schema {
        name: parsed.name,
        description: parsed.description,
        entity,
        frontmatter,
        reference_specs,
        body,
        workspace,
    })
}

fn build_body_rule(
    schema_path: &Path,
    idx: usize,
    entry: serde_yaml::Value,
    factories: &[Box<dyn BodyRuleFactory>],
    lookup: &HashMap<String, usize>,
) -> Result<Box<dyn BodyRule>, Error> {
    let (rule_id, params_json) = parse_rule_entry(schema_path, "body", idx, entry)?;
    let factory_idx = lookup.get(&rule_id).copied().ok_or_else(|| {
        Error::Schema(format!(
            "unknown body-rule id `{rule_id}` in body[{idx}] of {}",
            schema_path.display()
        ))
    })?;
    factories[factory_idx].build(&params_json)
}

fn build_links_rule(
    schema_path: &Path,
    idx: usize,
    entry: serde_yaml::Value,
    factories: &[Box<dyn WorkspaceRuleFactory>],
    lookup: &HashMap<String, usize>,
) -> Result<Box<dyn WorkspaceRule>, Error> {
    let (rule_id, params_json) = parse_rule_entry(schema_path, "links", idx, entry)?;
    let factory_idx = lookup.get(&rule_id).copied().ok_or_else(|| {
        Error::Schema(format!(
            "unknown link-rule id `{rule_id}` in links[{idx}] of {}",
            schema_path.display()
        ))
    })?;
    factories[factory_idx].build(&params_json)
}

/// Pull `(rule_id, params_json)` out of one rule-list entry, with section-aware error
/// messages so users learn which list owns the bad entry.
fn parse_rule_entry(
    schema_path: &Path,
    section: &str,
    idx: usize,
    entry: serde_yaml::Value,
) -> Result<(String, serde_json::Value), Error> {
    let serde_yaml::Value::Mapping(mut map) = entry else {
        return Err(Error::Schema(format!(
            "{section}[{idx}] in {} must be a mapping",
            schema_path.display()
        )));
    };

    let rule_id_value = map.remove("rule").ok_or_else(|| {
        Error::Schema(format!(
            "{section}[{idx}] in {} is missing the required `rule` key",
            schema_path.display()
        ))
    })?;
    let rule_id = rule_id_value
        .as_str()
        .ok_or_else(|| {
            Error::Schema(format!(
                "{section}[{idx}] in {}: `rule` must be a string",
                schema_path.display()
            ))
        })?
        .to_string();

    let params_yaml = serde_yaml::Value::Mapping(map);
    let params_json = serde_json::to_value(&params_yaml).map_err(|e| {
        Error::Schema(format!(
            "{section}[{idx}] params yaml→json in {}: {e}",
            schema_path.display()
        ))
    })?;

    Ok((rule_id, params_json))
}

fn build_body_lookup(factories: &[Box<dyn BodyRuleFactory>]) -> HashMap<String, usize> {
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

/// Lookup table for the `links:` block. Each `links.*` factory id maps to its index
/// under both the canonical id (`links.relative_path`) and the kebab-case shortform
/// with the `links.` prefix stripped (`relative-path`). Mirrors `build_body_lookup`.
///
/// Workspace-rule factories whose id does not start with `links.` are intentionally
/// omitted — they are not addressable from the `links:` block. Today the runtime
/// `types.entity_ref` rule is the only such case, and it is installed implicitly
/// via [`mdtype_rules_stdlib::install_type_checks`], not via the YAML loader.
fn build_links_lookup(factories: &[Box<dyn WorkspaceRuleFactory>]) -> HashMap<String, usize> {
    let mut map: HashMap<String, usize> = HashMap::new();
    for (i, factory) in factories.iter().enumerate() {
        let id = factory.id();
        let Some(short) = id.strip_prefix("links.") else {
            continue;
        };
        map.insert(id.to_string(), i);
        let kebab = short.replace('_', "-");
        map.entry(kebab).or_insert(i);
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
    entity: Option<String>,
    #[serde(default)]
    frontmatter: Option<serde_yaml::Value>,
    #[serde(default)]
    body: Vec<serde_yaml::Value>,
    #[serde(default)]
    links: Vec<serde_yaml::Value>,
    /// Legacy detection. The `workspace:` block was removed; if a schema still
    /// declares one we surface a precise migration error instead of silently
    /// dropping its rules.
    #[serde(default)]
    workspace: Option<serde_yaml::Value>,
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::Arc;

    use std::path::PathBuf;

    use mdtype_core::{
        BodyRule, BodyRuleFactory, Diagnostic, Error, ParsedDocument, Requirements, SchemaSource,
        Workspace, WorkspaceRule, WorkspaceRuleFactory,
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

    struct DummyLinkRule;
    impl WorkspaceRule for DummyLinkRule {
        fn id(&self) -> &'static str {
            "links.relative_path"
        }
        fn requires(&self) -> Requirements {
            Requirements::default()
        }
        fn check(&self, _ws: &Workspace, _scope: &[PathBuf], _out: &mut Vec<Diagnostic>) {}
    }

    struct DummyLinkFactory;
    impl WorkspaceRuleFactory for DummyLinkFactory {
        fn id(&self) -> &'static str {
            "links.relative_path"
        }
        fn build(&self, _params: &serde_json::Value) -> Result<Box<dyn WorkspaceRule>, Error> {
            Ok(Box::new(DummyLinkRule))
        }
    }

    fn dummy_link_registry() -> Vec<Box<dyn WorkspaceRuleFactory>> {
        vec![Box::new(DummyLinkFactory) as Box<dyn WorkspaceRuleFactory>]
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

        let src = YamlSchemaSource::new(
            dir.path().join(CONFIG_FILE_NAME),
            Arc::new(Vec::new()),
            Arc::new(Vec::new()),
        );
        let entries = src.load().expect("load");

        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        assert_eq!(entry.glob, "posts/**/*.md");
        assert_eq!(entry.schema.name, "blog-post");
        assert_eq!(entry.schema.description.as_deref(), Some("A post."));
        let fm = entry
            .schema
            .frontmatter
            .as_ref()
            .expect("frontmatter present");
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
        let schema = load_schema_file(&schema_path, &factories, &[]).expect("load");
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
        let schema = load_schema_file(&schema_path, &factories, &[]).expect("load");
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
        let result = load_schema_file(&schema_path, &factories, &[]);
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
    fn link_rule_resolves_via_kebab_alias() {
        let dir = tempdir().expect("tempdir");
        let schema_path = dir.path().join("post.yaml");
        fs::write(
            &schema_path,
            "name: post\nlinks:\n  - rule: relative-path\n",
        )
        .unwrap();

        let body_factories: Vec<Box<dyn BodyRuleFactory>> = Vec::new();
        let link_factories = dummy_link_registry();
        let schema =
            load_schema_file(&schema_path, &body_factories, &link_factories).expect("load");
        assert_eq!(schema.workspace.len(), 1);
        assert_eq!(schema.workspace[0].id(), "links.relative_path");
    }

    #[test]
    fn link_rule_resolves_via_canonical_id() {
        let dir = tempdir().expect("tempdir");
        let schema_path = dir.path().join("post.yaml");
        fs::write(
            &schema_path,
            "name: post\nlinks:\n  - rule: links.relative_path\n",
        )
        .unwrap();

        let body_factories: Vec<Box<dyn BodyRuleFactory>> = Vec::new();
        let link_factories = dummy_link_registry();
        let schema =
            load_schema_file(&schema_path, &body_factories, &link_factories).expect("load");
        assert_eq!(schema.workspace.len(), 1);
    }

    #[test]
    fn unknown_link_rule_is_a_schema_error() {
        let dir = tempdir().expect("tempdir");
        let schema_path = dir.path().join("post.yaml");
        fs::write(
            &schema_path,
            "name: post\nlinks:\n  - rule: not-a-real-link-rule\n",
        )
        .unwrap();

        let body_factories: Vec<Box<dyn BodyRuleFactory>> = Vec::new();
        let link_factories = dummy_link_registry();
        let result = load_schema_file(&schema_path, &body_factories, &link_factories);
        match result {
            Err(Error::Schema(msg)) => {
                assert!(
                    msg.contains("not-a-real-link-rule") && msg.contains("link-rule"),
                    "expected error to mention the rule id and link-rule context, got: {msg}"
                );
            }
            Err(other) => panic!("expected Error::Schema, got {other}"),
            Ok(_) => panic!("expected error, got Ok"),
        }
    }

    #[test]
    fn legacy_workspace_block_is_rejected_with_migration_hint() {
        let dir = tempdir().expect("tempdir");
        let schema_path = dir.path().join("post.yaml");
        fs::write(
            &schema_path,
            "name: post\nworkspace:\n  - rule: links.relative_path\n",
        )
        .unwrap();

        let body_factories: Vec<Box<dyn BodyRuleFactory>> = Vec::new();
        let link_factories = dummy_link_registry();
        let result = load_schema_file(&schema_path, &body_factories, &link_factories);
        match result {
            Err(Error::Schema(msg)) => {
                assert!(
                    msg.contains("`workspace:` block was removed") && msg.contains("`links:`"),
                    "expected migration hint, got: {msg}"
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
