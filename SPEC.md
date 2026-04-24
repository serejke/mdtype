# mdtype — Specification

## Overview

`mdtype` is a type checker for Markdown. It verifies that `.md` files conform to schemas declared in YAML. A schema covers two halves of a file: the YAML **frontmatter** (validated as JSON Schema) and the **body** (validated by declarative rules over the CommonMark AST — required sections, section ordering, forbidden constructs).

It ships as a small Rust CLI designed to be wired into pre-commit hooks, CI, or LLM agent stop hooks as a blocking gate: exit non-zero on violations, emit structured JSON that downstream tools can act on.

The tool is a **minimal extension of Markdown**. It does not transform, render, rewrite, or format files. It only validates.

---

## Philosophy (non-negotiable)

1. **Minimalistic.** Do one thing: validate markdown against schemas, emit diagnostics, exit non-zero on failure. Nothing else.
2. **Composable and modular.** Every layer is a trait. External crates extend without patching core.
3. **Unix-style.** Small CLI. Stable exit codes. JSON for machines, pretty for humans. Pipe-friendly.
4. **Minimal docs.** README reads like a man page. `--help` is the primary interface.
5. **Reuse, don't reinvent.** Frontmatter validation IS JSON Schema via the `jsonschema` crate. Markdown parsing IS CommonMark via `comrak`. No bespoke reimplementation.

---

## Architecture

```
                     .mdtype.yaml (glob → schema map)
                              │
                              ▼
                     ┌──────────────────┐
                     │  SchemaSource    │  trait — YAML/JSON/registry/…
                     └────────┬─────────┘
                              │ (glob, Schema) pairs
                              ▼
 files ──►  ┌─────────┐   ┌───────────┐   ┌──────────┐   ┌──────────┐
            │ Parser  │──►│ Validator │──►│Diagnostics│──►│ Reporter │──► stdout
            └─────────┘   └─────┬─────┘   └──────────┘   └──────────┘
                                │                               │
                          BodyRule trait                 human / json
                          (stdlib + ext)                 (pluggable)
```

Data flow is linear. Each arrow crosses a trait boundary; each node is swappable.

### Layers

| #   | Layer         | Trait / Type    | Purpose                                                                     |
| --- | ------------- | --------------- | --------------------------------------------------------------------------- |
| 1   | Schema source | `SchemaSource`  | Produce `(glob, Schema)` pairs from any backing store.                      |
| 2   | Schema model  | `Schema`        | In-memory: `frontmatter` (JSON Schema) + `body` (`Vec<Box<dyn BodyRule>>`). |
| 3   | Parser        | `parse_file`    | `.md` bytes → `(FrontmatterValue, MarkdownAst)`. One path, no leakage.      |
| 4   | Validator     | `Validator`     | `Schema + ParsedFile → Vec<Diagnostic>`. Deterministic order.               |
| 5   | Reporter      | `Reporter`      | `&[Diagnostic] → bytes on a Write`. Two built-ins: `human`, `json`.         |
| 6   | CLI           | `mdtype` binary | Thin wire-up. Every flag maps 1:1 to a library call.                        |

### Rule 1 of composition

`mdtype-core` depends on **no** concrete schema source, rule implementation, or reporter. It only defines traits, shared types, the parser, and the validator runtime. Everything else lives in sibling crates.

---

## Cargo Workspace

```
mdtype/
├── Cargo.toml                  # workspace manifest
├── rust-toolchain.toml         # pinned stable (MSRV 1.89)
├── rustfmt.toml
├── clippy.toml
├── README.md                   # one-screen man-page style
├── LICENSE-MIT
├── .gitignore
├── docs/
│   ├── schema.md               # schema file format reference
│   ├── rules.md                # every built-in body rule
│   ├── json-schema.md          # JSON output contract (versioned)
│   └── extending.md            # writing a custom BodyRule / SchemaSource / Reporter
├── crates/
│   ├── mdtype-core/            # traits, schema model, parser, validator
│   ├── mdtype-schema-yaml/     # YAML SchemaSource (default)
│   ├── mdtype-rules-stdlib/    # built-in BodyRule implementations
│   ├── mdtype-reporter-human/  # pretty, colored, grouped-by-file
│   ├── mdtype-reporter-json/   # versioned structured output
│   └── mdtype/                 # CLI binary
├── examples/
│   └── blog-site/              # canonical fixture (see Example Domain)
└── tests/
    └── golden/                 # end-to-end golden tests over examples/blog-site
```

### Crate dependency graph

```
mdtype (bin)
  ├── mdtype-core
  ├── mdtype-schema-yaml  ──► mdtype-core
  ├── mdtype-rules-stdlib ──► mdtype-core
  ├── mdtype-reporter-human ──► mdtype-core
  └── mdtype-reporter-json  ──► mdtype-core
```

`mdtype-core` has **zero** dependencies on sibling crates. Downstream users can pull `mdtype-core` + only the pieces they want.

---

## Core Types

All types live in `mdtype-core`.

### `Schema`

```rust
pub struct Schema {
    pub name: String,
    pub description: Option<String>,
    pub frontmatter: Option<serde_json::Value>,  // JSON Schema, draft 2020-12
    pub body: Vec<Box<dyn BodyRule>>,
}
```

### `Diagnostic`

```rust
pub struct Diagnostic {
    pub file: PathBuf,
    pub line: Option<usize>,       // 1-indexed; None for whole-file issues
    pub rule: &'static str,        // stable id, e.g. "frontmatter.required"
    pub severity: Severity,        // Error | Warning (v1: always Error)
    pub message: String,
    pub fixit: Option<Fixit>,      // hint only — mdtype never rewrites files
}

pub enum Fixit {
    AddFrontmatterField { field: String, hint: Option<String> },
    DeleteLine { line: usize },
    AppendSection { heading: String, after: Option<String> },
    Custom { name: String, payload: serde_json::Value },
}
```

### `BodyRule`

```rust
pub trait BodyRule: Send + Sync {
    /// Stable identifier, e.g. "body.required_sections".
    fn id(&self) -> &'static str;

    /// Validate the parsed document; append diagnostics to `out`.
    fn check(&self, doc: &ParsedDocument, out: &mut Vec<Diagnostic>);
}
```

### `SchemaSource`

```rust
pub trait SchemaSource {
    /// Return the set of (glob, Schema) pairs this source provides.
    fn load(&self) -> Result<Vec<SchemaEntry>, Error>;
}

pub struct SchemaEntry {
    pub glob: String,        // globset-compatible pattern, relative to config root
    pub schema: Schema,
}
```

### `Reporter`

```rust
pub trait Reporter {
    fn report(
        &self,
        diagnostics: &[Diagnostic],
        summary: &Summary,
        out: &mut dyn io::Write,
    ) -> io::Result<()>;
}
```

### `Validator`

```rust
pub trait Validator {
    fn validate(&self, file: &ParsedDocument, schema: &Schema) -> Vec<Diagnostic>;
}
```

A default `CoreValidator` ships in `mdtype-core`. Users can swap it.

---

## Schema File Format

Schemas are YAML files. Each file defines exactly one schema.

```yaml
name: blog-post
description: |
  A blog post under content/posts/. Must have title, date, tags, author;
  body must open with a `## Summary` section.

frontmatter:
  # Plain JSON Schema, draft 2020-12. Reuse everything the spec gives you.
  type: object
  required: [title, date, tags, author]
  additionalProperties: false
  properties:
    title: { type: string, minLength: 1, description: "Post title." }
    date:
      {
        type: string,
        format: date,
        description: "Publication date (YYYY-MM-DD).",
      }
    tags:
      {
        type: array,
        items: { type: string },
        minItems: 1,
        description: "Topic tags.",
      }
    author: { type: string, description: "Author name or handle." }
    draft: { type: boolean, description: "Hide from production site." }

body:
  # List of rule invocations. Each entry is { rule: <id>, <params...> }.
  - rule: forbid-h1
  - rule: required-sections
    sections: [Summary]
  - rule: section-order
    order: [Summary, Background, Details, Conclusion]
    mode: relaxed # required sections must appear in this order; optional ones skipped
  - rule: forbidden-sections
    sections: [TODO, Scratch]
```

### Root glob-map config

`.mdtype.yaml` at the project root, found by walking up from cwd:

```yaml
# Each entry is either { glob, schema: <path-to-schema-file> } or inline.
rules:
  - glob: "content/posts/**/*.md"
    schema: schemas/blog-post.yaml
  - glob: "content/docs/**/*.md"
    schema: schemas/doc-page.yaml

# Optional: baseline applied to every match. Composed *before* specific schemas.
base:
  frontmatter:
    type: object
    required: [title]
    properties:
      title: { type: string, minLength: 1 }
```

### Per-file schema override

A file may opt into a non-default schema via its own frontmatter:

```yaml
---
title: Draft — internal RFC
schema: schemas/rfc.yaml # relative to config root
---
```

When `schema:` is present, the glob-matched schema is **replaced**, not merged.

---

## Built-in Body Rules (v1)

All four live in `mdtype-rules-stdlib`. Each rule is a single file, trivially copy-pasteable as a template.

| Rule id                   | Parameters                                 | Description                                                                                                           |
| ------------------------- | ------------------------------------------ | --------------------------------------------------------------------------------------------------------------------- |
| `body.forbid_h1`          | (none)                                     | Disallow any top-level `# Heading`. Use when the filename is the title.                                               |
| `body.required_sections`  | `sections: [String]`                       | Assert that each named `##` (level-2) heading exists somewhere in the body.                                           |
| `body.section_order`      | `order: [String]`, `mode: strict\|relaxed` | Assert the ordering of required sections. Strict = no extra sections between; relaxed = order of named sections only. |
| `body.forbidden_sections` | `sections: [String]`                       | Assert that the named headings do **not** appear.                                                                     |

All section matching is by exact heading text, case-sensitive. Level defaults to H2; a future parameter can relax this.

---

## CLI

```
mdtype [OPTIONS] [PATHS]...

ARGS:
  PATHS                    Files or directories to validate. Default: current dir.

OPTIONS:
  -c, --config <FILE>      Path to .mdtype.yaml. Default: walk up from cwd.
      --schema <FILE>      Validate every PATH against this schema, ignoring the glob map.
  -f, --format <FORMAT>    human | json.  Default: human (if stdout is a tty) else json.
      --no-color           Disable colored output (human reporter only).
      --quiet              Suppress the summary line.
  -h, --help
  -V, --version
```

### Exit codes

| Code | Meaning                                                    |
| ---- | ---------------------------------------------------------- |
| 0    | All files clean.                                           |
| 1    | At least one diagnostic of severity Error.                 |
| 2    | Config or schema file is invalid; no validation performed. |

### Examples

```
# Validate the whole repo using .mdtype.yaml discovered by walking up.
$ mdtype

# Validate a specific directory with an explicit config.
$ mdtype --config .mdtype.yaml content/posts

# Validate a single file against a single schema, ignoring the glob map.
$ mdtype --schema schemas/blog-post.yaml content/posts/2026-01-hello.md

# Machine-readable output for a pre-commit hook.
$ mdtype --format json
```

---

## JSON Output Contract (public, versioned)

Documented in `docs/json-schema.md`. Treated as an API from day one.

```json
{
  "version": "1",
  "summary": {
    "files_scanned": 42,
    "files_with_errors": 1,
    "errors": 3,
    "warnings": 0
  },
  "diagnostics": [
    {
      "file": "content/posts/hello.md",
      "line": null,
      "rule": "frontmatter.required",
      "severity": "error",
      "message": "missing required field 'author'",
      "fixit": {
        "kind": "AddFrontmatterField",
        "field": "author",
        "hint": "string"
      }
    }
  ]
}
```

Contract rules:

- `version` is a required top-level string. Breaking changes bump it to `"2"`, never in place.
- Every diagnostic has a non-null `rule` id. Rule ids are stable — renames bump `version`.
- Unknown fields on output are never emitted. Consumers may ignore unknown fields for forward-compat.
- Snapshot tests in `tests/golden/` pin the exact output shape.

---

## Example Domain — `examples/blog-site/`

The canonical fixture used in all spec examples, README, and end-to-end tests. Generic static-site layout:

```
examples/blog-site/
├── .mdtype.yaml
├── schemas/
│   ├── blog-post.yaml
│   └── doc-page.yaml
└── content/
    ├── posts/
    │   ├── 2026-01-hello-world.md         # valid
    │   ├── 2026-02-missing-author.md      # fails frontmatter.required
    │   └── 2026-03-stray-h1.md            # fails body.forbid_h1
    └── docs/
        └── getting-started.md             # valid
```

Running `mdtype` from `examples/blog-site/` exits `1` with three diagnostics across two files. Running it from a clean subset (`content/posts/2026-01-hello-world.md`) exits `0`.

---

## Build & Commands

| Command                                                    | Purpose                            |
| ---------------------------------------------------------- | ---------------------------------- |
| `cargo build`                                              | Build all crates in the workspace. |
| `cargo test`                                               | Run unit + integration tests.      |
| `cargo run -p mdtype -- <args>`                            | Run the CLI during development.    |
| `cargo fmt --all`                                          | Format code. Enforced.             |
| `cargo clippy --all-targets --all-features -- -D warnings` | Lint. Enforced in CI.              |
| `cargo install --path crates/mdtype`                       | Install the CLI locally.           |

---

## Versions

- **Rust (MSRV):** 1.89, pinned via `rust-toolchain.toml`.
- **Edition:** 2021.
- **Crate versions** (target ranges, resolve actuals on first build):
  - `serde` 1, `serde_json` 1, `serde_yaml` 0.9
  - `jsonschema` 0.20+ (draft 2020-12)
  - `comrak` 0.28+
  - `globset` 0.4
  - `clap` 4 (derive feature)
  - `anyhow` 1, `thiserror` 1
  - `owo-colors` 4 (for the human reporter)
  - `insta` 1 (dev-only, snapshot tests)

---

## Non-Goals (v1)

- **No autofix.** Diagnostics may carry a `fixit` _hint_; mdtype does not rewrite files.
- **No rendering or formatting.** Not a Markdown formatter, not `prettier`.
- **No repo-wide rewrites** (no "rename field across all files").
- **No config discovery magic** beyond walking up for `.mdtype.yaml`.
- **No watch mode, daemon, LSP, shell completions** in v1.
- **No custom schema DSL.** Frontmatter validation is JSON Schema — full stop.
- **No heading-depth-limit or word-count rules** in v1 (straightforward additions later).

---

## Future Extensions (post-v1)

- Additional built-in rules: `heading-depth-limit`, `word-count`, `link-integrity`, `code-fence-language-required`.
- Autofix mode that emits patch files or applies them with `--write`.
- `mdtype init` subcommand to scaffold `.mdtype.yaml` + example schemas.
- Inline schema embedding (`schema: { ... full schema ... }` in frontmatter) for self-contained files.
- Watch mode, LSP, shell completions.
- Parallel validation (`--jobs N`) for large repos.
- Remote schema sources (`schema: https://.../blog-post.yaml`).

---

## Acceptance Criteria for v1

1. `cargo build` succeeds on the workspace with zero warnings.
2. `cargo clippy --all-targets --all-features -- -D warnings` passes.
3. `cargo test` passes, including the `tests/golden/` suite over `examples/blog-site/`.
4. Running the CLI on `examples/blog-site/`:
   - Clean files produce no diagnostics and exit 0.
   - Broken files produce the expected diagnostics and exit 1.
   - A malformed `.mdtype.yaml` exits 2.
5. `mdtype --format json` emits output matching `docs/json-schema.md` (snapshot-tested).
6. README fits on one screen and contains exactly one runnable example.
7. `docs/extending.md` shows a working external `BodyRule` in under 50 lines of Rust.
