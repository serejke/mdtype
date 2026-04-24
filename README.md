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
$ cat .mdtype.yaml
rules:
  - glob: "content/posts/**/*.md"
    schema: schemas/blog-post.yaml

$ mdtype
content/posts/2026-02-hello.md:
  frontmatter: missing required field 'author'
  body:L1: forbidden top-level H1 '# Hello'

2 errors in 1 file
```

Exit codes: `0` clean, `1` violations, `2` config error.

For machine-readable output in hooks and CI, pass `--format json`.

## Docs

- [`docs/schema.md`](docs/schema.md) — schema file format
- [`docs/rules.md`](docs/rules.md) — built-in body rules
- [`docs/json-schema.md`](docs/json-schema.md) — JSON output contract
- [`docs/extending.md`](docs/extending.md) — custom rules, sources, reporters

## License

MIT OR Apache-2.0.
