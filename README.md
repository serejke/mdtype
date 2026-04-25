# mdtype

A type checker for Markdown.

Declare the shape of your `.md` files — required frontmatter fields, required body sections, forbidden constructs — in YAML. Point `mdtype` at a directory. It tells you which files don't conform.

JSON Schema is to JSON what `mdtype` is to Markdown.

## Install

**From crates.io:**

```
cargo install mdtype
```

**Prebuilt binary** (Linux x86_64 + aarch64, macOS x86_64 + aarch64, Windows x86_64):

Grab the archive matching your platform from the latest [GitHub release](https://github.com/serejke/mdtype/releases/latest), unpack, and put the `mdtype` binary on your `PATH`. Each archive ships with a `.sha256` sidecar for integrity-checking.

**With `cargo binstall`** (fast, no source build):

```
cargo binstall mdtype
```

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
