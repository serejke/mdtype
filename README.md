# mdtype

A type checker for Markdown.

Declare the shape of your `.md` files in YAML. Point `mdtype` at a directory. It reports every file that doesn't conform — frontmatter, body structure, and cross-file links — in one pass. JSON Schema is to JSON what `mdtype` is to Markdown.

## Use cases

- **Blog or content site** — enforce frontmatter (title, date, tags, author), required body sections (Summary, Conclusion), section order, and that every `[link](other-post.md)` and `#anchor` actually resolves before merge.
- **Obsidian vault or knowledge base** — catch broken `[[wikilinks]]`, basenames that are silently ambiguous between two notes, and `[[Note#Heading]]` references whose heading was renamed away.
- **Engineering specs and RFCs** — require a stable layout (`## Motivation`, `## Design`, `## Open Questions`), forbid stray `## TODO` headings from leaking into shipped docs, verify every cross-referenced spec exists in the repo.
- **LLM-agent stop gate** — a coding agent writing Markdown can't declare "done" while `mdtype` exits non-zero. JSON output feeds the agent its remaining violations so it iterates to clean.

## What it checks

Three layers, mix and match per schema. Every diagnostic carries a stable `rule` id so machines and humans can both act on it.

### Frontmatter shape

JSON Schema (draft 2020-12) over the YAML frontmatter block.

```yaml
frontmatter:
  type: object
  required: [title, date, author]
  additionalProperties: false
  properties:
    date: { type: string, format: date }
    author: { type: string, minLength: 1 }
```

```text
content/posts/draft.md
  error[frontmatter.schema] missing required field 'author'
  error[frontmatter.schema] field 'date': expected string, found 42
```

### Body structure

Declarative rules over the parsed CommonMark AST.

```yaml
body:
  - rule: forbid-h1
  - rule: required-sections
    sections: [Summary, Conclusion]
  - rule: section-order
    order: [Summary, Background, Details, Conclusion]
  - rule: forbidden-sections
    sections: [TODO, Scratch, WIP]
```

```text
content/posts/draft.md
  error[body.required_sections] missing H2 section 'Conclusion'; add it as '## Conclusion'
  error[body.forbidden_sections] line 14: H2 section 'TODO' is not allowed (forbidden by schema)
```

### Cross-file links

Resolved against the walked file set, not the local filesystem — a target that exists outside the directory you scanned is reported as missing, the same way a type checker would treat an out-of-module symbol.

```yaml
workspace:
  - rule: links.relative_path
  - rule: links.obsidian_vault
    on_ambiguous: error
```

```text
content/posts/draft.md
  error[links.relative_path] line 7: link target 'setup.md' has no heading matching anchor '#install'
  error[links.obsidian_vault] line 11: wikilink target 'Twin' is ambiguous; equally-shortest matches: a/Twin.md, b/Twin.md
```

A blog uses frontmatter + body + `links.relative_path`. An Obsidian vault uses frontmatter + `links.obsidian_vault`. A docs site uses all four. Runnable layouts under [`examples/blog-site/`](examples/blog-site/) and [`examples/wiki-vault/`](examples/wiki-vault/).

## Run

```text
$ cat examples/blog-site/.mdtype.yaml
rules:
  - glob: "content/posts/**/*.md"
    schema: schemas/blog-post.yaml
  - glob: "content/docs/**/*.md"
    schema: schemas/doc-page.yaml

$ mdtype --format human --no-color examples/blog-site/
examples/blog-site/content/posts/2026-02-missing-author.md
  error[frontmatter.schema] missing required field 'author'

examples/blog-site/content/posts/2026-03-stray-h1.md
  error[body.forbid_h1] line 8: top-level heading '# Stray H1 — should fail body.forbid_h1' is not allowed; use '## Stray H1 — should fail body.forbid_h1' or rely on the file title

mdtype: 2 errors across 2 files (5 files scanned)
```

Exit codes: `0` clean, `1` violations, `2` config error. JSON output (the default in non-tty contexts) follows the versioned contract in [`docs/json-schema.md`](docs/json-schema.md). `mdtype --help` lists every flag.

## Quickstart

Install the binary, then let your coding agent wire up the rest of the project for you:

```sh
git clone https://github.com/serejke/mdtype && cd mdtype && cargo install --path crates/mdtype
npx skills add serejke/mdtype --skill setup-mdtype
```

Then, inside Claude Code / Codex / Cursor:

```
/setup-mdtype
```

The skill writes `.mdtype.yaml` + a starter schema, registers mdtype with `CLAUDE.md` or `AGENTS.md` so future agent sessions treat it as a blocking gate, runs a first sweep, and offers to install the pre-commit + Stop hooks. See [`skills/README.md`](skills/README.md).

Prefer a manual setup? Read [Install](#install) below.

## Install

Build from source (MSRV 1.89, pinned via `rust-toolchain.toml`):

```
git clone https://github.com/serejke/mdtype
cd mdtype
cargo install --path crates/mdtype
```

Don't have Rust? `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh` first, then re-run.

## Agent skill

The [`/setup-mdtype`](skills/setup-mdtype/SKILL.md) skill (shown in [Quickstart](#quickstart)) follows the [`npx skills`](https://github.com/vercel-labs/skills) convention and works with Claude Code, Codex, Cursor, and ~40 other coding agents. See [`skills/README.md`](skills/README.md) for per-agent install paths, global install, and the manual-copy fallback.

## Hooks

Drop-in scripts at [`hooks/`](hooks/):

- **`pre-commit`** — block any `git commit` that would land non-conforming Markdown. One-line install: `./hooks/install.sh /path/to/your/project`.
- **`claude-code-stop`** — Claude Code Stop hook. Blocks the agent from declaring "done" while diagnostics exist; feeds the JSON report back so it can fix and retry.
- **`.pre-commit-hooks.yaml`** at the repo root — for users of the [pre-commit framework](https://pre-commit.com).

See [`hooks/README.md`](hooks/README.md) for install + env-var configuration.

## Examples

- [`examples/blog-site/`](examples/blog-site/) — canonical fixture: posts with frontmatter, body rules, and `links.relative_path` for cross-post and post-to-doc references.
- [`examples/wiki-vault/`](examples/wiki-vault/) — Obsidian-style vault using `links.obsidian_vault` with shortest-path resolution, parent-suffix disambiguation, and anchor matching against raw heading text.
- [`examples/multi-folder/`](examples/multi-folder/) — schema-per-folder pattern. One root config, multiple globs, each pointing at a different schema (knowledge note / daily journal / folder-entry).

## Docs

- [`docs/schema.md`](docs/schema.md) — schema file format
- [`docs/rules.md`](docs/rules.md) — built-in body rules
- [`docs/json-schema.md`](docs/json-schema.md) — JSON output contract
- [`docs/error-messages.md`](docs/error-messages.md) — diagnostic message style guide
- [`docs/integrations.md`](docs/integrations.md) — pre-commit, CI, agent stop hook
- [`docs/extending.md`](docs/extending.md) — custom rules, sources, reporters

## Project

- [`CHANGELOG.md`](CHANGELOG.md) — per-release notes
- [`CONTRIBUTING.md`](CONTRIBUTING.md) — how to set up a working tree and propose a change

## License

MIT.
