# Implementation Plan

Reference: [SPEC.md](./SPEC.md)

Each phase is a self-contained unit of work. Complete one phase fully before moving to the next. Commit after every meaningful step within a phase. A new context window can pick up from any unchecked item and know exactly what to do.

---

## Phase 1: Workspace Skeleton

Goal: a compiling Cargo workspace with all six crates, trait signatures defined, `cargo build` green.

### 1.1 Repo hygiene

- [x] Confirm `Cargo.toml`, `rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`, `.gitignore`, `LICENSE-MIT` exist from the scaffold.
- [x] Confirm `crates/mdtype-core`, `crates/mdtype-schema-yaml`, `crates/mdtype-rules-stdlib`, `crates/mdtype-reporter-human`, `crates/mdtype-reporter-json`, `crates/mdtype` directories exist with `Cargo.toml` + `src/lib.rs` (or `src/main.rs` for the bin).
- [x] Verify: `cargo build --workspace` succeeds with zero warnings.

### 1.2 Core types

- [x] In `mdtype-core`, define `Diagnostic`, `Severity`, `Fixit`, `Summary` as per SPEC.md §Core Types.
- [x] Define trait signatures: `BodyRule`, `SchemaSource`, `Reporter`, `Validator`.
- [x] Define `Schema` and `SchemaEntry` structs.
- [x] Define `ParsedDocument` (holds frontmatter `serde_json::Value` + AST handle).
- [x] Add `mdtype_core::Error` with `thiserror`.
- [x] Verify: `cargo build -p mdtype-core` passes; `cargo clippy -p mdtype-core -- -D warnings` passes.

### 1.3 Parser module in `mdtype-core`

- [x] Implement `parse_file(path: &Path) -> Result<ParsedDocument, Error>`:
  - Split the YAML frontmatter (between `---` fences at top) from the body.
  - Parse frontmatter with `serde_yaml` into `serde_json::Value`.
  - Parse body with `comrak` into an AST.
- [x] Record 1-indexed line offset of the body start so rule diagnostics can report absolute line numbers.
- [x] Unit test: a fixture with frontmatter and a fixture without; both parse.
- [x] Verify: `cargo test -p mdtype-core` passes.

### 1.4 Default `CoreValidator`

- [x] Implement `CoreValidator` in `mdtype-core` that:
  1. Runs frontmatter through `jsonschema` if the schema declares one.
  2. Runs each `BodyRule::check` in order, appending diagnostics.
  3. Returns diagnostics sorted by (file, line, rule).
- [x] Unit test against a hand-built `Schema` with an empty body rule list.
- [x] Verify: `cargo test -p mdtype-core` passes.

---

## Phase 2: YAML Schema Source + CLI Wiring (MVP frontmatter-only)

Goal: `mdtype` runs against real files, validates frontmatter only, prints a human report, exits correctly. No body rules yet.

### 2.1 `mdtype-schema-yaml`

- [x] Implement `YamlSchemaSource { config_path, root }` that reads `.mdtype.yaml`.
- [x] Parse the `rules:` entries; each points to a schema file path.
- [x] For each entry, load the referenced YAML schema file (frontmatter block parsed into `serde_json::Value`; body block parsed as a list of rule invocations — leave body rules empty for now, error on unknown rule ids in a later phase).
- [x] Implement `config_walk_up(start: &Path) -> Option<PathBuf>` to find the nearest `.mdtype.yaml`.
- [x] Verify: unit test loads a fixture config + schema file pair and returns a `SchemaEntry`.

### 2.2 `mdtype-reporter-human`

- [x] Implement `HumanReporter` grouping diagnostics by file.
- [x] Use `owo-colors` behind a `--no-color` toggle and tty detection.
- [x] Format matches SPEC.md §CLI examples.
- [x] Snapshot test with `insta` on a fixed diagnostic list.
- [x] Verify: `cargo test -p mdtype-reporter-human` passes.

### 2.3 CLI plumbing

- [x] Define clap args in `crates/mdtype/src/main.rs` matching SPEC.md §CLI exactly.
- [x] Implement the pipeline:
  1. Load config (explicit `--config` or walk-up).
  2. Construct `YamlSchemaSource`, call `load()`.
  3. Walk PATHS, collect `.md` files.
  4. For each file, glob-match against schema entries; pick the schema (respect per-file `schema:` override).
  5. Parse the file, run `CoreValidator`.
  6. Feed all diagnostics into the selected reporter.
  7. Exit 0/1/2 per the spec.
- [x] Implement per-file `schema:` override lookup.
- [x] Verify: `cargo run -p mdtype -- examples/blog-site/content/posts/2026-01-hello-world.md` exits 0 and prints nothing.

### 2.4 End-to-end smoke test

- [x] Create `examples/blog-site/` with `.mdtype.yaml`, `schemas/blog-post.yaml`, and the three fixture posts from SPEC.md §Example Domain. Body rules in the schema can be an empty list for now.
- [x] Create a golden test in `tests/golden/` that runs the CLI over `examples/blog-site/`, captures stdout+exit, compares via `insta`.
- [x] Verify: `cargo test` passes; frontmatter violations are detected end-to-end.

---

## Phase 3: Body Rule Stdlib (`mdtype-rules-stdlib`)

Goal: ship the four v1 body rules and wire them through the YAML schema loader.

### 3.1 Rule registry plumbing

- [x] In `mdtype-rules-stdlib`, expose `fn register_stdlib() -> Vec<Box<dyn BodyRuleFactory>>` (or equivalent).
- [x] Define `BodyRuleFactory` trait in `mdtype-core` that parses a YAML node into `Box<dyn BodyRule>`. Each stdlib rule implements its own factory.
- [x] Update `YamlSchemaSource` to accept a list of factories and look up rule ids when parsing the `body:` block. Unknown ids produce a `config error → exit 2`.
- [x] Verify: `cargo build --workspace` green.

### 3.2 `body.forbid_h1`

- [x] Implement rule + factory.
- [x] Unit tests: H1 present → diagnostic with line; H1 absent → no diagnostic.
- [x] Add to `docs/rules.md` (one paragraph + example).

### 3.3 `body.required_sections`

- [x] Implement (exact-text H2 match by default).
- [x] Unit tests: all present, one missing, none present.
- [x] Add to `docs/rules.md`.

### 3.4 `body.section_order`

- [x] Implement both `strict` and `relaxed` modes.
- [x] Unit tests covering: correct order, inverted order (relaxed & strict), extra section between (relaxed ignores, strict flags), missing required section.
- [x] Add to `docs/rules.md`.

### 3.5 `body.forbidden_sections`

- [x] Implement.
- [x] Unit tests: forbidden present → diagnostic with line; absent → clean.
- [x] Add to `docs/rules.md`.

### 3.6 Wire into CLI + fixture

- [x] In `crates/mdtype/src/main.rs`, register stdlib factories with `YamlSchemaSource` on startup.
- [x] Update `examples/blog-site/schemas/blog-post.yaml` to use all four rules per SPEC.md §Schema File Format.
- [x] Update the golden test fixtures so body-rule violations are exercised.
- [x] Verify: golden tests pass; broken fixtures trigger the expected rule ids.

---

## Phase 4: JSON Reporter + Stop-Hook Integration Story

Goal: stable machine-readable output + documentation showing how to wire mdtype into hooks.

### 4.1 `mdtype-reporter-json`

- [x] Define serde-serializable wire types that mirror SPEC.md §JSON Output Contract exactly.
- [x] Implement `JsonReporter` emitting pretty-printed JSON when stdout is a tty, compact otherwise.
- [x] Include `version: "1"` unconditionally.
- [x] Snapshot test with `insta` — this is the public contract.

### 4.2 CLI `--format json`

- [x] Wire `--format json` to select `JsonReporter`.
- [x] Default format: `human` if stdout is a tty, else `json`.
- [x] Golden test: `mdtype --format json examples/blog-site/` matches a pinned snapshot.

### 4.3 Docs: JSON contract + hooks

- [x] Write `docs/json-schema.md` documenting every field. Call out the versioning rule.
- [x] Write a short `docs/integrations.md` (or a section in README) with three recipes:
  1. Pre-commit hook (single `mdtype` invocation over staged `.md` files).
  2. CI job (runs on every PR; fails the build on exit 1).
  3. Generic LLM agent stop hook (reads the JSON, feeds diagnostics back to the model, re-runs until clean). Tool-agnostic — no product names.

---

## Phase 5: Docs + Extension Guide

Goal: the composable story made concrete.

### 5.1 `docs/schema.md`

- [x] Document every field in the schema file format (name, description, frontmatter, body). Include a full working example. Link to the JSON Schema 2020-12 spec for frontmatter.

### 5.2 `docs/extending.md`

- [x] Write a working example: a new `BodyRule` (e.g., `heading_depth_limit`) in a downstream crate in under 50 lines.
- [x] Show how to register a custom `SchemaSource` (e.g., JSON-backed).
- [x] Show how to swap the reporter.
- [x] Include a `cargo.toml` snippet showing which `mdtype-*` crates the downstream crate depends on.

### 5.3 README

- [x] One-screen overview. One runnable example. Link to `docs/`. No marketing, no emojis, no screenshots.
- [x] Verify: README fits in ~80 lines.

---

## Phase 6: Polish + Release Prep

### 6.1 Lint & format sweep

- [x] `cargo fmt --all --check` clean.
- [x] `cargo clippy --all-targets --all-features -- -D warnings` clean.
- [x] `cargo test --workspace` clean.

### 6.2 Full run verification

- [x] `cargo run -p mdtype -- examples/blog-site/` exits 1 and lists the expected diagnostics.
- [x] `cargo run -p mdtype -- examples/blog-site/content/posts/2026-01-hello-world.md` exits 0.
- [x] `cargo run -p mdtype -- --format json examples/blog-site/` produces JSON matching the documented contract.
- [x] Malformed `.mdtype.yaml` → exit 2 with a clear error.
- [x] `mdtype --help` prints the flag table from SPEC.md §CLI.

### 6.3 Acceptance checklist from SPEC.md

- [x] Walk SPEC.md §Acceptance Criteria for v1 and tick each item.

### 6.4 CI

- [x] Add `.github/workflows/ci.yml` running `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test --workspace` on push and PR.
- [ ] Verify: CI green on an empty PR. _(deferred — requires the user to push and observe the GitHub Actions run.)_

---

## Phase 7: `mdtype-tests` crate (high-level integration tests)

Goal: a dedicated workspace crate that owns every high-level tool test: scenario fixtures, golden files, and reporter-parity checks. Adding a new scenario is "drop a folder in, regenerate the snapshot" — no Rust changes per scenario. Starts from the simplest happy-path, grows down the matrix.

### 7.1 Crate skeleton

- [x] Create `crates/mdtype-tests/` as a workspace member. Library is empty (or pure helpers); the unit of work is `tests/`.
- [x] Move `crates/mdtype/tests/golden.rs` and its snapshots under the new crate (`crates/mdtype-tests/tests/blog_site.rs` keeps the canonical example covered).
- [x] Verify: `cargo test -p mdtype-tests` passes the moved blog-site goldens.

### 7.2 Fixture-driven harness

- [x] Implement `crates/mdtype-tests/tests/fixtures.rs`: walks `crates/mdtype-tests/fixtures/<scenario>/`, runs the real `mdtype` binary against each, asserts exit code + stdout snapshot for each (one snapshot per `--format`).
- [x] Each `<scenario>/` is a self-contained mini-project: its own `.mdtype.yaml`, `schemas/`, `content/`, and a sibling `expected/{exit_code, stdout.human, stdout.json}`.
- [x] Use `escargot` or `env!("CARGO_BIN_EXE_mdtype")` so the harness exercises the real CLI surface.

### 7.3 Simple scenarios (start here)

- [x] `simple-clean/` — single valid file; exit `0`; empty diagnostics.
- [x] `simple-missing-required/` — single file missing one required frontmatter field; exit `1`; one diagnostic.
- [x] `simple-stray-h1/` — single file with a `# H1`; exit `1`; one `body.forbid_h1` diagnostic.

### 7.4 Configuration scenarios

- [x] `config-missing/` — directory with no `.mdtype.yaml`. Exit `2`.
- [x] `config-malformed/` — `.mdtype.yaml` with invalid YAML. Exit `2`.
- [x] `config-unknown-rule/` — schema references a body-rule id not in the registry. Exit `2`.
- [x] `config-explicit/` — invocation with `--config` overriding walk-up.

### 7.5 Frontmatter scenarios

- [x] `frontmatter-wrong-type/` — type mismatch (array required, scalar given).
- [x] `frontmatter-additional-properties/` — schema with `additionalProperties: false` and a file with an extra field.
- [x] `frontmatter-absent/` — file with no `---` block where the schema requires frontmatter.
- [x] `frontmatter-malformed/` — leading `---` opened but never closed.

### 7.6 Body-rule scenarios

- [x] `body-required-sections/` — clean / one-missing / all-missing triple.
- [x] `body-section-order-strict/` — correct / inverted / extra-between / missing.
- [x] `body-section-order-relaxed/` — same matrix; extras tolerated.
- [x] `body-forbidden-sections/` — clean + broken pair.
- [x] `multi-rule/` — one file violating multiple rules; verifies stable `(file, line, rule)` ordering.

### 7.7 Selection scenarios

- [x] `per-file-schema-override/` — file with `schema:` in its frontmatter; verifies replacement (not merge).
- [x] `non-md-files/` — `.txt`, `.png`, `.html` alongside `.md`; verifies only `.md` enters the pipeline.
- [x] `empty-tree/` — directory with no `.md` files; exit `0`, no diagnostics.

### 7.8 Reporter parity

- [x] Every scenario above snapshots **both** `--format human --no-color` and `--format json`. Divergence between the two is a regression in exactly one reporter.
- [x] Verify: `cargo test --workspace` passes; deliberately breaking any scenario fixture fails the right snapshot with a clear diff and the correct exit-code mismatch.

---

## Phase 8: LLM-friendly error messages

Goal: every diagnostic `message` is short, declarative, and structured enough for an agent to know what to fix without scraping prose. The JSON contract already gives `rule`, `file`, `line`, `severity`, `fixit`; this phase upgrades the `message` field to match.

### 8.1 Style guide + audit

- [x] Write `docs/error-messages.md`: imperative voice, "expected X, found Y" shape where applicable, mention the offending value verbatim, no parenthetical aside, no Markdown.
- [x] Snapshot the current message text for every rule (one fixture per rule) so the rewrite is observable in diffs. _(Phase 7.x fixtures already snapshot every rule's current message; the rewrite in 8.2 will show as diffs in those goldens.)_

### 8.2 Rewrite each rule's messages

- [x] `frontmatter.schema` — surface the JSON pointer (`/author`) and the expected type/format. Replace `"\"X\" is a required property"` with `missing required field 'X' (expected string)` or similar.
- [x] `body.forbid_h1` — include the offending heading text: `top-level heading '# Stray H1' is not allowed; use '## Stray H1' or rely on the file title`.
- [x] `body.required_sections` — include the section name and the rule's expected level: `missing H2 section 'Summary'; add it as '## Summary'`.
- [x] `body.section_order` — include the offending and expected positions: `H2 section 'Background' appears at line N but should appear after 'Summary'`.
- [x] `body.forbidden_sections` — include the heading text and the policy: `H2 section 'TODO' is not allowed (forbidden by schema); remove the heading and its content`.
- [x] `mdtype.parse` — name the failure class up front: `frontmatter parse failed: missing closing '---' fence on line N`.

### 8.3 Verify

- [x] All snapshots updated; tests green.
- [x] Eyeball-test on examples/blog-site/: every diagnostic an agent could see is unambiguous about what to change.

---

## Appendix: Session Discipline

- Always read `CLAUDE.md` before starting a session.
- Always read `PLAN.md` to find the first unchecked item.
- Work on exactly one item at a time. Mark `[x]` when verified, not when written.
- Commit after every 1.x / 2.x sub-task, not at the end of a phase.
- When a phase is complete, stop. Do not start the next phase in the same session unless the context is fresh and the scope is still trivially small.
