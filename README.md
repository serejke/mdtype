# mdtype

A compiler for Markdown projects.

`mdtype` type-checks a directory of `.md` files the way `tsc` type-checks a directory of `.ts` files: per-file shape (frontmatter via JSON Schema, body via declarative rules) and project-wide structure (link resolution and typed cross-document references). One binary, stable exit codes, JSON output for machines, pretty output for humans. It is built to slot into pre-commit hooks, CI, or LLM-agent stop hooks as a blocking gate.

## The problem

Conventions in a Markdown directory live in maintainers' heads, not in the files themselves. Without a checker:

- An ADR's `discussed_in` field starts pointing at the wrong kind of document, and nobody notices.
- A heading section gets renamed and every `[link](./that-page.md#old-anchor)` silently rots.
- `## Open Questions` migrates to the bottom of the template; older docs keep it at the top; nothing reads as drift until a reviewer happens to spot it.
- Frontmatter fields gain or lose meaning over time; old files retain stale schemas and new tooling quietly skips them.

Drift compounds. Readers stop trusting the structure, which means the structure stops being structure. `mdtype` makes the conventions explicit and **executable**: the schema is the source of truth, and any file that doesn't conform fails the build.

## What it checks

Two layers, both driven by YAML schemas.

### Per-file checks

For every Markdown file matched by a glob, `mdtype` runs:

- **Frontmatter shape** — JSON Schema 2020-12 over the YAML block. `type`, `required`, `properties`, `additionalProperties`, `enum`, `format`, conditional schemas — the full vocabulary. See [`docs/schema.md`](docs/schema.md).
- **Body structure** — declarative rules over the parsed CommonMark AST. Built-ins: `forbid-h1`, `required-sections`, `section-order` (strict and relaxed), `forbidden-sections`. See [`docs/rules.md`](docs/rules.md).

### Project-wide checks

Once every file is parsed, `mdtype` runs cross-file rules against the assembled workspace:

- **Link resolution.** `links.relative_path` resolves inline Markdown links (`[text](path.md)`) and validates `#anchor` fragments against the target file's heading slugs. Schemes like `http`/`mailto` are skipped. See [`docs/rules.md`](docs/rules.md#linksrelative_path).
- **Typed cross-document references.** Schemas can declare `entity:` (a kind name attached to every file the schema matches) and inline `x-entity:` annotations on frontmatter properties (this field's values must be paths to files of a specific kind). The compiler walks these annotations at schema-load time and type-checks every cross-file pointer in the workspace. Five diagnostic templates cover the failure cases: missing target, wrong type, untyped target, anchor in reference, and field-shape mismatch. See [`docs/types.md`](docs/types.md).
- **Wikilinks (optional dialect).** For projects that author with `[[Target]]` syntax, the `mdtype-rules-obsidian` crate ships `links.obsidian_vault` — exact-path → basename → shortest-path resolution with anchor matching against raw heading text.

The two layers compose. Frontmatter validation, body rules, link resolution, and typed references all run in a single pass against the same parsed AST.

## Quickstart

Install the binary:

```sh
git clone https://github.com/serejke/mdtype
cd mdtype
cargo install --path crates/mdtype
```

(Rust 1.89 or later. Don't have Rust? `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh` first, then re-run.)

Drop a `.mdtype.yaml` at the project root mapping globs to schemas, write a schema per document class, and run:

```sh
mdtype docs/
```

Exit codes: `0` clean, `1` violations, `2` config error. Default format resolves to `human` for tty stdout, `json` otherwise. `mdtype --help` lists every flag.

A worked example, against the canonical fixture in this repo:

```text
$ cat examples/blog-site/.mdtype.yaml
rules:
  - glob: "content/posts/**/*.md"
    schema: schemas/blog-post.yaml
  - glob: "content/authors/**/*.md"
    schema: schemas/author.yaml
  - glob: "content/docs/**/*.md"
    schema: schemas/doc-page.yaml

$ mdtype --format human --no-color examples/blog-site/
examples/blog-site/content/posts/2026-02-missing-author.md
  error[frontmatter.schema] missing required field 'author'

examples/blog-site/content/posts/2026-03-stray-h1.md
  error[body.forbid_h1] line 8: top-level heading '# Stray H1 — should fail body.forbid_h1' is not allowed; use '## Stray H1 — should fail body.forbid_h1' or rely on the file title

examples/blog-site/content/posts/2026-06-broken-references.md
  error[types.entity_ref] field 'author_profile': link target '../authors/ghost.md' not found in workspace

mdtype: 3 errors across 3 files (8 files scanned)
```

Three different layers caught three different kinds of drift in one run.

## Use cases

`mdtype` is for any project where a Markdown directory is a first-class artifact whose structure matters.

### `docs/` directory in a software project

Schemas pin frontmatter (page id, author, last reviewed), forbid `## TODO` from leaking into shipped pages, require `## Setup` and `## Reference` sections in the right order, and ensure every `[setup guide](setup.md)` link resolves. CI gate: `mdtype docs/`. New contributors learn the structure from the diagnostics, not from a wiki page about the wiki.

```yaml
# schemas/doc-page.yaml
name: doc-page
entity: doc
frontmatter:
  type: object
  required: [title, last_reviewed]
  properties:
    title: { type: string, minLength: 1 }
    last_reviewed: { type: string, format: date }
body:
  - rule: forbid-h1
  - rule: required-sections
    sections: [Overview]
  - rule: forbidden-sections
    sections: [TODO, FIXME]
```

### Architecture Decision Records (ADRs)

ADRs reference meeting transcripts, RFCs, and prior ADRs. Without typing, those references are just strings. With typing, the compiler enforces that `discussed_in` points at meeting transcripts, `supersedes` points at earlier ADRs, and `references_rfc` points at RFCs. Cross-document drift becomes a build error.

```yaml
# schemas/adr.yaml
name: adr
entity: adr
frontmatter:
  type: object
  required: [title, status, date, discussed_in]
  properties:
    status: { enum: [proposed, accepted, superseded] }
    discussed_in:
      type: array
      items:
        type: string
        x-entity: [meeting-transcript, adr]
body:
  - rule: required-sections
    sections: [Context, Decision, Consequences]
```

A new ADR that points at the wrong file class:

```text
docs/adr/0042-use-postgres.md
  error[types.entity_ref] field 'discussed_in': link target '../rfcs/0001-multi-tenant.md': expected entity one of 'meeting-transcript', 'adr', got 'rfc'
```

The full machinery is documented in [`docs/types.md`](docs/types.md).

### RFC repositories

`status` enum, `## Motivation` / `## Design` / `## Open Questions` required and ordered, `references` field typed as RFC pointers, `## TODO` and `## Scratch` forbidden in shipped RFCs. Reviewers can lean on the compiler instead of the boilerplate-checker hat.

### Engineering handbooks, runbooks, on-call playbooks

Every runbook declares `entity: runbook` with required `## Symptoms`, `## Diagnosis`, `## Mitigation`, `## Rollback`. Cross-references between runbooks are typed (`linked_runbooks: [{type: string, x-entity: runbook}]`). When a runbook is renamed or retired, every typed reference to it surfaces as a diagnostic — no silent rot.

### API documentation, spec repositories

Front-matter pins the API version and stability tier; body rules require `## Request`, `## Response`, `## Errors`; typed references link example pages back to their spec page. Multi-author drift gets caught at PR time, not after a release.

### Multi-author technical writing

Any directory with more than two authors and more than two months on it. The schema is the social contract; `mdtype` enforces it.

## Integrations

`mdtype` is designed to plug into existing pipelines.

- **Git pre-commit.** [`hooks/pre-commit`](hooks/pre-commit) blocks commits with non-conforming Markdown. One-line install: `./hooks/install.sh /path/to/your/project`. Users of the [pre-commit framework](https://pre-commit.com) point at the repo's `.pre-commit-hooks.yaml`. See [`hooks/README.md`](hooks/README.md).
- **CI.** Drop `mdtype docs/` (or `mdtype .`) into a job. Non-zero exit fails the job. JSON output (the default in non-tty contexts) is documented in [`docs/json-schema.md`](docs/json-schema.md). See [`docs/integrations.md`](docs/integrations.md) for GitHub Actions, GitLab CI, and CircleCI snippets.
- **LLM-agent stop hook.** [`hooks/claude-code-stop`](hooks/claude-code-stop) blocks Claude Code from declaring "done" while diagnostics exist, feeding the JSON report back so the agent fixes and retries. The same JSON contract works with any agent harness that supports stop hooks.
- **Setup skill.** `npx skills add serejke/mdtype --skill setup-mdtype` installs a `/setup-mdtype` slash command for Claude Code, Codex, Cursor, and ~40 other coding agents. The skill writes a starter `.mdtype.yaml` + schema, registers `mdtype` in `CLAUDE.md` / `AGENTS.md` so future agent sessions treat it as a blocking gate, runs a first sweep, and offers to install the pre-commit + Stop hooks. See [`skills/README.md`](skills/README.md).

## Extending

Every layer of `mdtype` is a trait in `mdtype-core`:

- A **body rule** examines a single parsed AST and appends diagnostics. ~50 lines for a typical rule.
- A **link (cross-file) rule** examines the whole indexed file set (paths, headings, links, frontmatter, entities) and judges its scope. Link rules are listed under a schema's `links:` block.
- A **schema source** produces `(glob, Schema)` pairs from any backing store — YAML on disk is the default, but JSON, an HTTP service, or a hand-built table are all fair game.
- A **reporter** writes diagnostics to any `io::Write`. Built-ins are `human` and `json`; SARIF, JUnit, or Slack-formatted output is one trait impl away.

External crates plug new behaviour in without patching core or the CLI. See [`docs/extending.md`](docs/extending.md) for working examples of each extension point.

## Examples in this repo

- [`examples/blog-site/`](examples/blog-site/) — posts, authors, doc pages. Demonstrates frontmatter, body rules, `links.relative_path`, and typed `author_profile` references via `entity:` + `x-entity:`.
- [`examples/wiki-vault/`](examples/wiki-vault/) — Obsidian-style vault using `links.obsidian_vault` (shortest-path resolution, parent-suffix disambiguation, anchor matching against raw heading text).
- [`examples/multi-folder/`](examples/multi-folder/) — schema-per-folder pattern. One root config, multiple globs, each pointing at a different schema.

## Docs

- [`docs/schema.md`](docs/schema.md) — schema file format
- [`docs/types.md`](docs/types.md) — `entity:`, `x-entity:`, typed cross-document references
- [`docs/rules.md`](docs/rules.md) — built-in body and link rules
- [`docs/json-schema.md`](docs/json-schema.md) — JSON output contract
- [`docs/error-messages.md`](docs/error-messages.md) — diagnostic message style guide
- [`docs/integrations.md`](docs/integrations.md) — pre-commit, CI, agent stop hook
- [`docs/extending.md`](docs/extending.md) — custom rules, sources, reporters

## Project

- [`CHANGELOG.md`](CHANGELOG.md) — per-release notes
- [`CONTRIBUTING.md`](CONTRIBUTING.md) — set up a working tree, run the gates, propose a change

## License

MIT.
