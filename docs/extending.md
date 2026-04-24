# Extending mdtype

Every layer of `mdtype` is a trait in [`mdtype-core`](../crates/mdtype-core). External crates plug new behaviour in without patching core or the CLI: add a body rule, swap the schema source, swap the reporter — three pure additions, no forks.

This page walks each extension point with a working example.

## What you depend on

For a downstream crate that ships rules, sources, or reporters, depend only on `mdtype-core`. Pull in sibling crates (`mdtype-rules-stdlib`, `mdtype-schema-yaml`, the reporters) only if you want to reuse their concrete pieces.

```toml
# Cargo.toml of your downstream crate
[dependencies]
mdtype-core         = "0.1"
serde               = { version = "1", features = ["derive"] }
serde_json          = "1"

# Optional, only if you reuse the canonical YAML loader / stdlib rules / built-in reporters:
# mdtype-schema-yaml  = "0.1"
# mdtype-rules-stdlib = "0.1"
# mdtype-reporter-json = "0.1"
```

`mdtype-core` re-exports the bits of `comrak` you need (`Arena`, `nodes`, the parser entry point) so a rule crate does not need a direct `comrak` dep.

## 1. A new body rule (under 50 lines)

A `BodyRule` looks at the parsed AST and appends `Diagnostic`s. A `BodyRuleFactory` builds one from the YAML parameters declared in a schema's `body:` block.

`heading_depth_limit` — fail if any heading is deeper than `max`:

```rust
use mdtype_core::nodes::NodeValue;
use mdtype_core::{
    BodyRule, BodyRuleFactory, Diagnostic, Error, ParsedDocument, Severity,
};
use serde::Deserialize;

pub const ID: &str = "body.heading_depth_limit";

pub struct Rule {
    pub max: u8,
}

impl BodyRule for Rule {
    fn id(&self) -> &'static str { ID }

    fn check(&self, doc: &ParsedDocument, out: &mut Vec<Diagnostic>) {
        let offset = doc.body_line_offset.saturating_sub(1);
        for node in doc.ast.descendants() {
            let data = node.data.borrow();
            let NodeValue::Heading(h) = &data.value else { continue };
            if h.level > self.max {
                out.push(Diagnostic {
                    file: doc.path.clone(),
                    line: Some(data.sourcepos.start.line + offset),
                    rule: ID,
                    severity: Severity::Error,
                    message: format!("heading depth {} exceeds max {}", h.level, self.max),
                    fixit: None,
                });
            }
        }
    }
}

#[derive(Deserialize)]
struct Params { max: u8 }

pub struct Factory;
impl BodyRuleFactory for Factory {
    fn id(&self) -> &'static str { ID }
    fn build(&self, params: &serde_json::Value) -> Result<Box<dyn BodyRule>, Error> {
        let p: Params = serde_json::from_value(params.clone())
            .map_err(|e| Error::Schema(format!("{ID}: {e}")))?;
        Ok(Box::new(Rule { max: p.max }))
    }
}
```

Wire it up by passing your factory alongside the stdlib ones to whichever `SchemaSource` is in use:

```rust
let mut factories = mdtype_rules_stdlib::register_stdlib();
factories.push(Box::new(my_rules::heading_depth_limit::Factory));
let factories = std::sync::Arc::new(factories);
```

YAML reference:

```yaml
body:
  - rule: heading-depth-limit # or body.heading_depth_limit
    max: 3
```

## 2. A custom `SchemaSource`

`YamlSchemaSource` is one implementation. The trait is open — back schemas with JSON, a database, an HTTP service, or a hand-built table for tests.

A minimal in-memory source useful for testing:

```rust
use mdtype_core::{Error, Schema, SchemaEntry, SchemaSource};

pub struct InMemorySource(pub Vec<SchemaEntry>);

impl SchemaSource for InMemorySource {
    fn load(&self) -> Result<Vec<SchemaEntry>, Error> {
        // The CLI consumes (glob, Schema) pairs; cloning isn't always cheap because
        // Schema owns trait-object body rules. For tests, build a fresh source per call.
        Err(Error::Other("InMemorySource is move-only; rebuild per call".into()))
    }
}
```

The realistic pattern is a source that reads from your backing store on each `load()`. If you want JSON-backed schemas, mirror `mdtype-schema-yaml`'s shape:

```rust
use std::path::PathBuf;
use std::sync::Arc;
use mdtype_core::{BodyRuleFactory, Error, Schema, SchemaEntry, SchemaSource};

pub struct JsonSchemaSource {
    pub config_path: PathBuf,
    pub root: PathBuf,
    pub factories: Arc<Vec<Box<dyn BodyRuleFactory>>>,
}

impl SchemaSource for JsonSchemaSource {
    fn load(&self) -> Result<Vec<SchemaEntry>, Error> {
        // 1. Read self.config_path as JSON.
        // 2. For each entry, read the referenced JSON schema file.
        // 3. Resolve `body:` rules through self.factories (mirror mdtype-schema-yaml).
        // 4. Return Vec<SchemaEntry>.
        unimplemented!()
    }
}
```

Hand the resulting source to `CoreValidator` exactly the way the CLI hands it the YAML one — no other changes needed.

## 3. Swap the reporter

A `Reporter` writes a `&[Diagnostic]` plus `Summary` to any `io::Write`. The two built-ins (`mdtype-reporter-human`, `mdtype-reporter-json`) are reference implementations; nothing stops you from emitting SARIF, TAP, JUnit, or Slack-formatted text.

A trivially small JUnit-ish reporter:

```rust
use std::io;
use mdtype_core::{Diagnostic, Reporter, Summary};

pub struct LinesReporter;

impl Reporter for LinesReporter {
    fn report(
        &self,
        diagnostics: &[Diagnostic],
        _summary: &Summary,
        out: &mut dyn io::Write,
    ) -> io::Result<()> {
        for d in diagnostics {
            writeln!(
                out,
                "{}:{}: {}: {}",
                d.file.display(),
                d.line.map_or(String::from("-"), |l| l.to_string()),
                d.rule,
                d.message,
            )?;
        }
        Ok(())
    }
}
```

If you want this picked up by the `mdtype` CLI rather than your own front-end, build your own thin binary that wires `clap → SchemaSource → CoreValidator → YourReporter`. The CLI in `crates/mdtype/src/main.rs` is ~300 lines and is the canonical template — copy it and replace the bits you care about.

## Why this stays small

`mdtype-core` declares the data model (`Diagnostic`, `Severity`, `Fixit`, `Summary`, `ParsedDocument`, `Schema`, `SchemaEntry`) and the four traits (`BodyRule`, `BodyRuleFactory`, `SchemaSource`, `Reporter`). It contains the parser and the default `CoreValidator`. **It depends on no sibling crate.** Anything you build downstream is a peer of the stdlib, not a fork of it.
