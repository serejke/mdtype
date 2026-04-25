# mdtype

A type checker for Markdown.

Declare the shape of your `.md` files — required frontmatter fields, required body sections, forbidden constructs — in YAML. Point `mdtype` at a directory. It tells you which files don't conform.

JSON Schema is to JSON what `mdtype` is to Markdown. Designed to plug into pre-commit hooks, CI, and LLM agent stop hooks as a blocking gate.

## Quickstart

Install the binary, then let your coding agent wire up the rest of the project for you:

```sh
cargo install mdtype                                  # 1. install the binary
npx skills add serejke/mdtype --skill setup-mdtype    # 2. install the /setup-mdtype agent skill
```

Then, inside Claude Code / Codex / Cursor:

```
/setup-mdtype
```

The skill writes `.mdtype.yaml` + a starter schema, registers mdtype with `CLAUDE.md` or `AGENTS.md` so future agent sessions treat it as a blocking gate, runs a first sweep, and offers to install the pre-commit + Stop hooks. See [`skills/README.md`](skills/README.md).

Prefer a manual setup? Read [Install](#install) + [Use](#use) below.

## Install

```
cargo install mdtype
```

That's it. Don't have Rust? `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh` first, then re-run.

**From source** (MSRV 1.89, pinned via `rust-toolchain.toml`):

```
git clone https://github.com/serejke/mdtype
cd mdtype
cargo install --path crates/mdtype
```

## Use

```
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

mdtype: 2 errors across 2 files (4 files scanned)
```

Exit codes: `0` clean, `1` violations, `2` config error.

`--format json` (the default in non-tty contexts) emits the versioned contract documented in [`docs/json-schema.md`](docs/json-schema.md). Use `mdtype --help` for the full flag table.

## Agent skill

The [`/setup-mdtype`](skills/setup-mdtype/SKILL.md) skill (shown in [Quickstart](#quickstart)) follows the [`npx skills`](https://github.com/vercel-labs/skills) convention and works with Claude Code, Codex, Cursor, and ~40 other coding agents. See [`skills/README.md`](skills/README.md) for per-agent install paths, global install, and the manual-copy fallback.

## Hooks

Drop-in scripts at [`hooks/`](hooks/):

- **`pre-commit`** — block any `git commit` that would land non-conforming Markdown. One-line install: `./hooks/install.sh /path/to/your/project`.
- **`claude-code-stop`** — Claude Code Stop hook. Blocks the agent from declaring "done" while diagnostics exist; feeds the JSON report back so it can fix and retry.
- **`.pre-commit-hooks.yaml`** at the repo root — for users of the [pre-commit framework](https://pre-commit.com).

See [`hooks/README.md`](hooks/README.md) for install + env-var configuration.

## Examples

- [`examples/blog-site/`](examples/blog-site/) — single-schema project; the canonical fixture.
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
