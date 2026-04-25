# Agent Instructions

`mdtype` v1 is shipped. The phased build plan that produced it lives in git history (commit `bd9a11d` and earlier). Use this file as the standing rulebook for any future change.

## Architectural Invariants (do not violate)

1. **Preserve the trait boundaries.** If a change requires hard-coding a concrete rule, schema source, or reporter into `mdtype-core`, that is a design smell — stop and reconsider.
2. **New rules go in `mdtype-rules-stdlib` or a downstream crate.** Never add rule logic to `mdtype-core`.
3. **Every diagnostic has a stable `rule` id.** The JSON output schema is a public contract — breaking changes bump `version`, never in place.
4. **Keep the CLI surface small.** New flags require justification in the commit message.
5. **Reuse, don't reinvent.** Frontmatter validation IS JSON Schema (via `jsonschema`). Markdown parsing IS CommonMark (via `comrak`). Do not write a new YAML parser, JSON Schema engine, or Markdown parser.
6. **No autofix.** Diagnostics may carry a `fixit` hint; `mdtype` never rewrites files.
7. **Neutral examples only.** All examples, fixtures, and docs use the generic `examples/blog-site/` domain. Never introduce examples drawn from a personal workflow, vault, or private product.

## Diagnostic Message Discipline

Every `Diagnostic.message` is consumed by humans **and** by LLM agents. Read [`docs/error-messages.md`](docs/error-messages.md) before editing or adding one. Snapshot tests in `crates/mdtype-tests/fixtures/` lock the wording per rule; regenerate via `UPDATE_FIXTURES=1 cargo test -p mdtype-tests --test fixtures`.

## Documentation Discipline

- README reads like a man page: one-screen overview, one runnable example, link to `docs/`. No marketing. No emojis. No screenshots.
- `mdtype --help` is the primary interface. Docs only repeat what is not obvious from help text.
- Every built-in rule gets a paragraph + example in `docs/rules.md`.
- Every schema file field gets documented in `docs/schema.md`.
- `docs/json-schema.md` is the versioned public JSON contract. Snapshot-tested. Never edit without bumping `version`.

## Workflow

- **One logical change per commit.** Conventional Commits format (`type(scope): subject`). No Claude Code footer / co-author line.
- **Tests are the source of truth for behaviour.** A regression should fail an existing test or get a new one — adding a feature without a test means the contract is unwritten.
- **Sweep before pushing.** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace` must all be green. CI in `.github/workflows/ci.yml` enforces the same.
- **Every fixture snapshots both reporters.** New scenarios in `crates/mdtype-tests/fixtures/` get their `expected/{exit_code,stdout.human,stdout.json}` regenerated together.

## Reference Files

- `README.md` — Man-page overview + install + one runnable example.
- `CHANGELOG.md` — Per-release notes (Keep a Changelog format).
- `CONTRIBUTING.md` — How to set up a working tree, run the gates, propose a change.
- `docs/` — Living reference: schema format, rule catalogue, JSON contract, error-message style guide, integration recipes, extension guide.
- `AGENTS.md` — Mirror pointer for agents that read `AGENTS.md` instead.

The product contract lives in the code + tests + `docs/`. There is no separate spec doc.

## Project Context

`mdtype` is a type checker for Markdown: a Rust CLI that validates `.md` files against YAML schemas (JSON Schema for frontmatter, declarative rules for body structure). It is deliberately minimal — one binary, stable exit codes, JSON output for machines, pretty output for humans — and every layer is a trait so external crates can extend without patching core. It is designed to plug into pre-commit hooks, CI, or LLM agent stop hooks as a blocking gate.
