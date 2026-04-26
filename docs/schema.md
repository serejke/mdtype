# Schema File Format

A `mdtype` schema is a YAML file that describes one document shape. The root config (`.mdtype.yaml`) maps glob patterns to schema files; each schema file is parsed into the in-memory [`Schema`](../crates/mdtype-core/src/schema.rs) used by the validator. This page documents every field.

For body rules referenced from the `body:` block, see [`docs/rules.md`](./rules.md). For the JSON output produced after validation, see [`docs/json-schema.md`](./json-schema.md).

## Top-level fields

| Field         | Type   | Required | Notes                                                                                                                                                             |
| ------------- | ------ | -------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `name`        | string | yes      | Stable identifier, surfaced in error messages and useful when more than one schema is loaded. Conventionally the file stem.                                       |
| `description` | string | no       | Free-form description; multi-line YAML strings are supported. Surfaced in human reports.                                                                          |
| `entity`      | string | no       | Entity name (a kind) attached to every file matched by this schema. Enables type-checked cross-file references — see [`docs/types.md`](./types.md).               |
| `frontmatter` | object | no       | A [JSON Schema 2020-12][jsonschema] document, validated against the file's parsed YAML frontmatter. May carry inline `x-entity` annotations; see `docs/types.md`. |
| `body`        | array  | no       | Ordered list of body-rule invocations. Empty or missing means no body checks.                                                                                     |
| `links`       | array  | no       | Ordered list of link-rule invocations (cross-file integrity checks). Empty or missing means no link checks.                                                       |

[jsonschema]: https://json-schema.org/specification-links#2020-12

`entity:` and inline `x-entity:` annotations together define a small type system for cross-document references. They are documented separately in [`docs/types.md`](./types.md). The schema loader walks the `frontmatter:` JSON Schema for `x-entity` keys on string-typed properties or array items and synthesises a `types.entity_ref` check that fires implicitly — there is no rule entry to enable.

## `frontmatter`

The value is passed unchanged to the [`jsonschema`](https://docs.rs/jsonschema) crate's draft-2020-12 validator. The full JSON Schema vocabulary is available — `type`, `required`, `properties`, `additionalProperties`, `format`, `enum`, `minLength`, `pattern`, conditional schemas, `$ref`, etc.

Notes:

- The frontmatter block in a Markdown file is parsed as YAML and converted to JSON before validation, so anything expressible in YAML must be expressible in JSON (no anchors-of-functions, no tagged scalars).
- Format keywords (`format: date`, `format: email`, …) are validated when supported by the underlying engine. Unsupported formats degrade to "string accepted".
- A frontmatter validation failure produces one diagnostic per JSON Schema error with `rule = "frontmatter.schema"`, `line = null`. The message text comes from the validator and is intended for humans, not pattern-matching.

## `body`

Each entry is a mapping with a required `rule` key plus rule-specific parameters:

```yaml
body:
  - rule: forbid-h1
  - rule: required-sections
    sections: [Summary, Conclusion]
  - rule: section-order
    order: [Summary, Background, Details, Conclusion]
    mode: relaxed
  - rule: forbidden-sections
    sections: [TODO, Scratch]
```

Both the canonical id (e.g. `body.forbid_h1`) and the kebab-case shortform (`forbid-h1`) are accepted in YAML; diagnostics always carry the canonical id. An unknown rule id raises a config error and the CLI exits `2`.

Rules execute in declaration order. Each contributes zero or more diagnostics; the validator then sorts the full list by `(file, line, rule)` for stable output.

## `links`

Link-integrity rules. Each entry is a mapping with a required `rule` key plus rule-specific parameters:

```yaml
links:
  - rule: relative-path
  - rule: obsidian-vault
    on_ambiguous: error
    check_anchors: true
```

Link rules run after every file is parsed. Each rule declares which fact kinds it needs (headings, inline links, wikilinks); the runner unions those declarations across the run, gathers the facts once, then judges each rule against the files attached to its schema. The same rule listed in two schemas with different parameters produces two independent rule instances — neither bleeds into the other.

Both the canonical id (e.g. `links.relative_path`) and the kebab-case shortform with the `links.` prefix stripped (`relative-path`) are accepted in YAML; diagnostics always carry the canonical id. The catalogue lives in [`docs/rules.md`](./rules.md). Unknown rule ids raise a config error.

> **Migrating from `workspace:`.** Earlier versions of `mdtype` listed link rules under a `workspace:` block. That block was removed; move the entries under `links:` and drop the `links.` prefix on rule ids. A schema that still declares `workspace:` fails to load with a precise migration hint.

## One root config per project (no merging)

`mdtype` walks **up** from the cwd looking for the nearest `.mdtype.yaml` and uses **only that one**. There is no merging across parent and child configs — the closest config wins, period. A `.mdtype.yaml` in a subdirectory silently shadows any ancestor `.mdtype.yaml` further up the tree.

The right pattern is therefore **one root `.mdtype.yaml` per project, with multiple globs**, not nested configs at every folder level:

```yaml
# .mdtype.yaml at the project root
rules:
  - glob: "Daily/**/*.md"
    schema: .mdtype/schemas/daily-note.yaml
  - glob: "Projects/*/Sprints/**/*Workstream.md"
    schema: .mdtype/schemas/workstream-note.yaml
  - glob: "**/*.md"
    schema: .mdtype/schemas/note.yaml
```

If `mdtype` detects descendant `.mdtype.yaml` files inside the loaded config's tree, it warns on stderr — those files are shadowed and have no effect. See [`examples/multi-folder/`](../examples/multi-folder/) for a complete worked layout.

### Glob matching within one config

Globs are matched in **declaration order**; the **first** glob that hits a given file wins. Order rules from most-specific to catch-all. There is no specificity heuristic — reorder the list and you change which schema applies. Globs follow Unix-shell semantics: `*` matches a single path segment (does not cross `/`); `**` traverses directories.

## Per-file schema override

A Markdown file may opt into a non-default schema by adding a `schema:` field to its own frontmatter:

```markdown
---
title: Internal RFC
schema: schemas/rfc.yaml
---

…body…
```

When `schema:` is present, the glob-matched schema is **replaced** (not merged) with the one at the given path. Relative paths resolve against the directory containing `.mdtype.yaml`.

## Full working example

`.mdtype.yaml`:

```yaml
rules:
  - glob: "content/posts/**/*.md"
    schema: schemas/blog-post.yaml
  - glob: "content/docs/**/*.md"
    schema: schemas/doc-page.yaml
```

`schemas/blog-post.yaml`:

```yaml
name: blog-post
description: |
  A blog post under content/posts/. Frontmatter must declare
  title, date, tags, author; the body must open with `## Summary`
  and not contain a top-level `# H1`.

frontmatter:
  type: object
  required: [title, date, tags, author]
  additionalProperties: false
  properties:
    title:
      type: string
      minLength: 1
    date:
      type: string
      format: date
    tags:
      type: array
      items: { type: string }
      minItems: 1
    author:
      type: string
    draft:
      type: boolean

body:
  - rule: forbid-h1
  - rule: required-sections
    sections: [Summary]
  - rule: section-order
    order: [Summary, Background, Details, Conclusion]
    mode: relaxed
  - rule: forbidden-sections
    sections: [TODO, Scratch]
```

A complete, runnable copy of this layout lives under [`examples/blog-site/`](../examples/blog-site/) and is exercised by the end-to-end golden tests in `crates/mdtype/tests/golden.rs`.
