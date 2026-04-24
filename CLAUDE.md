# Agent Instructions

## Workflow Rules

1. **Read PLAN.md first.** At the start of every session, read `PLAN.md` to find the current state of the project.
2. **Pick the next unfinished task.** Find the first unchecked (`- [ ]`) item in `PLAN.md`. Work on that phase/step only. Do NOT skip ahead or work on later phases.
3. **One task at a time.** Complete the current step fully before moving to the next. Do not partially implement multiple steps.
4. **Mark progress.** When a step is done and verified, update `PLAN.md` — change `- [ ]` to `- [x]` for that item.
5. **Git commit after every meaningful step.** After completing a substep (e.g., 1.1, 1.2) or a logical unit of work, stage the relevant files and commit with a clear message describing what was done. Do not batch multiple unrelated steps into one commit.
6. **Do not work beyond the current phase.** If all steps in the current phase are done, mark the phase complete and stop. The user will start a new session for the next phase if needed.
7. **Test before marking done.** If a step has a "Verify" note, run the verification before marking it complete. If verification fails, fix the issue before moving on.
8. **When making a Git commit, NEVER add a Claude Code footer or co-author line.**

## Architectural Invariants (do not violate)

1. **Preserve the trait boundaries.** If a change requires hard-coding a concrete rule, schema source, or reporter into `mdtype-core`, that is a design smell — stop and reconsider.
2. **New rules go in `mdtype-rules-stdlib` or a downstream crate.** Never add rule logic to `mdtype-core`.
3. **Every diagnostic has a stable `rule` id.** The JSON output schema is a public contract — breaking changes bump `version`, never in place.
4. **Keep the CLI surface small.** New flags require justification in the commit message.
5. **Reuse, don't reinvent.** Frontmatter validation IS JSON Schema (via `jsonschema`). Markdown parsing IS CommonMark (via `comrak`). Do not write a new YAML parser, JSON Schema engine, or Markdown parser.
6. **No autofix in v1.** Diagnostics may carry a `fixit` hint; mdtype never rewrites files.
7. **Neutral examples only.** All examples, fixtures, and docs use the generic `examples/blog-site/` domain. Never introduce examples drawn from a personal workflow, vault, or private product.

## Documentation Discipline

- README reads like a man page: one-screen overview, one runnable example, link to `docs/`. No marketing. No emojis. No screenshots.
- `mdtype --help` is the primary interface. Docs only repeat what is not obvious from help text.
- Every built-in rule gets a paragraph + example in `docs/rules.md`.
- Every schema file field gets documented in `docs/schema.md`.
- `docs/json-schema.md` is the versioned public JSON contract. Snapshot-tested. Never edit without bumping `version`.

## Reference Files

- `SPEC.md` — Full project specification. Immutable during the build.
- `PLAN.md` — Implementation phases with checkboxes. Source of truth for progress.
- `AGENTS.md` — Mirror of this file for agents that read `AGENTS.md` instead.

## Project Context

`mdtype` is a type checker for Markdown: a Rust CLI that validates `.md` files against YAML schemas (JSON Schema for frontmatter, declarative rules for body structure). It is deliberately minimal — one binary, stable exit codes, JSON output for machines, pretty output for humans — and every layer is a trait so external crates can extend without patching core. It is designed to plug into pre-commit hooks, CI, or LLM agent stop hooks as a blocking gate.
