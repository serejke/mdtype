# Changelog

All notable changes to `mdtype` are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/serejke/mdtype/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/serejke/mdtype/releases/tag/v0.1.0
