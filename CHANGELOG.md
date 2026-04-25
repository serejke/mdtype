# Changelog

All notable changes to `mdtype` are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] — 2026-04-26

Cross-file rule support: workspace pipeline + two link-integrity rules. Design rationale
in [`docs/proposals/0001-workspace-pipeline.md`](docs/proposals/0001-workspace-pipeline.md).

### Added

- **Workspace pipeline** in `mdtype-core` — `run_workspace(files, schemas, schema_idx)` parses every file in a single arena, builds a policy-free `Workspace` index (files / headings / links / frontmatter), runs every body rule (unchanged semantics), then runs every workspace rule against its scope.
- **`WorkspaceRule` and `WorkspaceRuleFactory` traits** alongside `BodyRule`. Workspace rules declare `Requirements` (what fact kinds they need) and judge `&Workspace` for their scope. The runner unions requirements across all enabled rules and configures both the parser and the extractors accordingly.
- **`links.relative_path`** workspace rule (`mdtype-rules-stdlib`) — resolves inline Markdown links against the source file's directory, validates `#anchor` fragments against the target's GitHub-flavored heading slugs, skips configurable URI schemes (`http`, `https`, `mailto`, `tel` by default).
- **`mdtype-rules-obsidian` crate** with `links.obsidian_vault` — resolves wikilinks (`[[Target]]`, `[[Target|Alias]]`, `[[Target#Heading]]`) using Obsidian's exact-path → basename → shortest-path policy. Equal-depth ambiguities surface as configurable diagnostics (`on_ambiguous: error | warn | first-match`).
- **`workspace:` block in schemas** — ordered list of workspace-rule invocations. Workspace rule ids are canonical-only in v0.2; the body kebab shortform does not extend.
- **`Schema::default()`** — adding new fields to `Schema` (such as the new `workspace`) without breaking every literal-construction site downstream. Migrate existing literals with `..Schema::default()` or by adding `workspace: Vec::new()`.
- **`parse_file_with_options`, `read_frontmatter`, `pub split_frontmatter`** — additive parser API so the runner can drive comrak options from rule `Requirements` and the CLI can pre-pass frontmatter without a body parse.
- **CLI `run_workspace` integration** — replaces the per-file parse-and-validate loop with a frontmatter pre-pass that resolves schemas (including `frontmatter.schema:` overrides) before the runner does the body parse. Pre-pass parse failures are surfaced as `mdtype.parse` diagnostics; the runner never re-attempts a parse the pre-pass already failed.
- **Three new fixtures** under `crates/mdtype-tests/fixtures/`: `links-relative-path/`, `links-obsidian-vault/`, `links-mixed/`. Each snapshots both reporters via the existing harness.
- **Two runnable examples**: `examples/blog-site/` extended with `links.relative_path` and a new `2026-04-cross-references.md` post; new `examples/wiki-vault/` demonstrates `links.obsidian_vault` resolution end-to-end.

### Changed

- **`load_schema_file` and `YamlSchemaSource::new` signatures** now take both factory registries (`Vec<Box<dyn BodyRuleFactory>>` and `Vec<Box<dyn WorkspaceRuleFactory>>`). Downstream callers must pass both — supply an empty vector for the kind they don't extend.
- **`Schema` gains a public `workspace: Vec<Box<dyn WorkspaceRule>>` field.** This is a source-level break for callers that build `Schema` with a struct literal. With the new `Default` impl, prefer `..Schema::default()` for forward compatibility.

### Notes

- The JSON output contract (`version: "1"`) is unchanged; new diagnostics carry new `rule` ids (`links.relative_path`, `links.obsidian_vault`) but the schema's documented shape is stable.

## [0.1.0] — 2026-04-25

Initial release. A type checker for Markdown: validates `.md` files against YAML schemas
(JSON Schema for frontmatter, declarative rules for body structure). Single binary, stable
exit codes, versioned JSON output.

### Added

- **Workspace** of seven crates with a strict trait boundary: `mdtype-core` (data model,
  parser, default validator) depends on no sibling. `mdtype-schema-yaml`,
  `mdtype-rules-stdlib`, `mdtype-reporter-human`, `mdtype-reporter-json`, the `mdtype`
  CLI, and `mdtype-tests` (end-to-end fixture suite) all sit on top.
- **CLI** — `mdtype [OPTIONS] [PATHS]...` with `-c/--config`, `--schema`, `-f/--format`,
  `--no-color`, `--quiet`. Exit codes `0` clean, `1` violations, `2` config error. Default
  format resolves to `human` for tty stdout, `json` otherwise.
- **YAML schema source** with config walk-up (`.mdtype.yaml`) and per-file `schema:`
  override. Both the canonical rule id (`body.forbid_h1`) and a kebab-case shortform
  (`forbid-h1`) are accepted in YAML.
- **Frontmatter validation** via the `jsonschema` crate (draft 2020-12). Diagnostics carry
  the JSON pointer of the offending field.
- **Body rules** (`mdtype-rules-stdlib`): `body.forbid_h1`, `body.required_sections`,
  `body.section_order` (strict + relaxed), `body.forbidden_sections`.
- **Reporters**: `human` (grouped by file, optional color, silent on success) and `json`
  (pretty for tty, compact otherwise). The JSON shape is the public contract — see
  [`docs/json-schema.md`](docs/json-schema.md).
- **LLM-friendly diagnostic messages**: every message is imperative, quotes the offending
  value, and names the expected shape. Style guide in
  [`docs/error-messages.md`](docs/error-messages.md).
- **Example fixture**: `examples/blog-site/` — canonical mini-project used in docs and end-
  to-end tests.
- **Integration test crate** `mdtype-tests` — fixture-driven harness over
  `crates/mdtype-tests/fixtures/<scenario>/` covering 17 scenarios. Each scenario
  snapshots both `--format human --no-color` and `--format json` so the reporters cannot
  drift apart silently. Regenerate via `UPDATE_FIXTURES=1 cargo test -p mdtype-tests`.
- **Documentation**: README (one-screen man-page), `docs/schema.md`, `docs/rules.md`,
  `docs/json-schema.md`, `docs/error-messages.md`, `docs/integrations.md`,
  `docs/extending.md`.
- **Distribution**: source-build only (`git clone && cargo install --path crates/mdtype`).
  The workspace is split into `mdtype-core` + four sibling crates so downstream rules /
  sources / reporters can depend on `mdtype-core` alone, but the project does not currently
  publish to crates.io — the binary is the product, the libraries are the extension surface.
- **Agent skill**: `setup-mdtype` follows the [`vercel-labs/skills`](https://github.com/vercel-labs/skills)
  convention. Install with `npx skills add serejke/mdtype --skill setup-mdtype`, then
  `/setup-mdtype` inside Claude Code / Codex / Cursor / 40-odd other agents to bootstrap
  config, schemas, and the agent's instruction file.
- **CI**: `.github/workflows/ci.yml` — `cargo fmt --all --check`, `cargo clippy --workspace
--all-targets --all-features -- -D warnings`, `cargo test --workspace`.

[Unreleased]: https://github.com/serejke/mdtype/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/serejke/mdtype/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/serejke/mdtype/releases/tag/v0.1.0
