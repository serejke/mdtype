# mdtype

A type checker for Markdown.

Declare the shape of your `.md` files — required frontmatter fields, required body sections, forbidden constructs — in YAML. Point `mdtype` at a directory. It tells you which files don't conform.

JSON Schema is to JSON what `mdtype` is to Markdown.

## Install

```
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
  error[frontmatter.schema] "author" is a required property

examples/blog-site/content/posts/2026-03-stray-h1.md
  error[body.forbid_h1] line 8: top-level H1 is not allowed

mdtype: 2 errors across 2 files (4 files scanned)
```

Exit codes: `0` clean, `1` violations, `2` config error.

`--format json` (the default in non-tty contexts) emits the versioned contract documented in [`docs/json-schema.md`](docs/json-schema.md). Use `mdtype --help` for the full flag table.

## Docs

- [`docs/schema.md`](docs/schema.md) — schema file format
- [`docs/rules.md`](docs/rules.md) — built-in body rules
- [`docs/json-schema.md`](docs/json-schema.md) — JSON output contract
- [`docs/integrations.md`](docs/integrations.md) — pre-commit, CI, agent stop hook
- [`docs/extending.md`](docs/extending.md) — custom rules, sources, reporters

## License

MIT.
