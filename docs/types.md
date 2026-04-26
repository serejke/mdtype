# Schema-derived type checks

`mdtype` runs three layers of checks against your Markdown files:

1. **Frontmatter shape** — declared via the schema's `frontmatter:` JSON Schema.
2. **Body structure** — declared via the schema's `body:` rule list.
3. **Cross-file edges and types** — declared either via the `workspace:` rule list (rules you opt into; see [`docs/rules.md`](./rules.md)) or via inline annotations on the schema itself. This page covers the third layer's **declaration-driven** subset.

Schema-derived type checks fire because a schema declares them inline — there is no rule entry to enable, no parameters to pass. The user surface is two YAML keywords (`entity:` and `x-entity:`) that together describe a typed graph; the runtime mechanics are hidden.

## `entity:` — naming the kind

A schema may declare an entity name for the files it matches:

```yaml
name: blog-post
entity: post

frontmatter:
  type: object
  required: [title]
  properties:
    title: { type: string }
```

Every file matched by this schema (via glob or per-file `schema:` override) has entity `post`. Other schemas can demand that their reference fields point at files with this name. The name is project-namespaced (no global registry) and need not be unique across schemas — two schemas declaring `entity: post` together attach files to the same class.

If `entity:` is present, its value must be a non-empty string. Anything else fails the schema load.

## `x-entity:` — typing a field's edges

Inside `frontmatter:`, a string-typed property whose value is a typed pointer carries `x-entity`:

```yaml
frontmatter:
  type: object
  properties:
    author_profile:
      type: string
      x-entity: author
```

The annotation says: every value of `author_profile` is a path that must resolve to a file whose entity is `author`. Resolution is path-based (relative to the source file's directory) and judged against the workspace, mirroring `links.relative_path`.

`x-entity` is a custom keyword. JSON Schema 2020-12 [requires validators to ignore unknown keywords](https://json-schema.org/draft/2020-12/json-schema-core#name-keywords) when they don't recognise them, so shape validation is unaffected: strings get string checks, arrays get array checks. The annotation rides alongside.

### Three forms

**Scalar** — single target, single value:

```yaml
properties:
  author_profile:
    type: string
    x-entity: author
```

**Array** — many targets of the same type:

```yaml
properties:
  reviewers:
    type: array
    items:
      type: string
      x-entity: author
```

The annotation rides on the items schema, where the _element_ type lives.

**Union** — value(s) may resolve to one of several entity kinds:

```yaml
properties:
  related:
    type: array
    items:
      type: string
      x-entity: [post, tag]
```

The `x-entity` value is an array of entity names. Each value in `related` must resolve to a file whose entity matches at least one of the alternatives.

### `x-entity` value rules

The annotation must be one of:

- a non-empty string (single target), or
- a non-empty array of non-empty, non-duplicate strings (union).

Anything else fails the schema load:

```yaml
x-entity: ""              # rejected: empty
x-entity: []              # rejected: empty array
x-entity: ["a", 42]       # rejected: non-string element
x-entity: ["a", "a"]      # rejected: duplicate
x-entity: 42              # rejected: not a string
```

### Where `x-entity` is allowed

Two positions inside the frontmatter JSON Schema:

- `properties.<name>` whose schema is `{ type: "string", x-entity: ... }`.
- `properties.<name>.items` whose schema is `{ type: "string", x-entity: ... }` (under a `type: "array"` parent).

Anywhere else is rejected at schema load. Specifically, **`$ref`, `oneOf`, `anyOf`, `allOf`, `if/then/else`, and `not` are not supported in v1**: if `x-entity` is reachable through any of those constructs, or if the schema uses `$ref` anywhere alongside an `x-entity` annotation, the schema fails to load with a precise error.

```
schema schemas/post.yaml: x-entity reachable through 'oneOf' at /properties/cover/oneOf
is not supported; declare the field directly under 'properties' with type 'string'
or 'array of string'.
```

This is **explicit failure**, not silent degradation. Schemas that use composition for _non-reference_ fields are unaffected — the rejection only fires when `x-entity` is hidden inside the unsupported construct (or when `$ref` and `x-entity` coexist anywhere in the same schema).

## Diagnostics

Every schema-derived type-check failure surfaces under the diagnostic id `types.entity_ref` with one of five message templates:

| Class                       | Message template                                                                         |
| --------------------------- | ---------------------------------------------------------------------------------------- |
| `field_invalid`             | `field 'F': expected string or array of strings, found {kind}`                           |
| `target_anchor_unsupported` | `field 'F': link target 'P' carries an anchor; entity references must be document-level` |
| `target_missing`            | `field 'F': link target 'P' not found in workspace`                                      |
| `target_untyped`            | `field 'F': link target 'P' has no declared entity, expected {expected}`                 |
| `target_type`               | `field 'F': link target 'P': expected entity {expected}, got '{actual}'`                 |

`{expected}` is `'NAME'` for a single target, `one of 'A', 'B'` for a union. `{actual}` is the singular entity of the resolved target file.

Diagnostics carry no `line` — frontmatter values do not have stable per-element source locations in v1. The field name in every message disambiguates which value triggered the diagnostic.

The `types.` prefix marks the diagnostic as belonging to the **type system** alongside `frontmatter.schema`. It does not sit under `links.*` because the user does not enable it as a rule — they declare types and the check follows.

## Worked example

```yaml
# schemas/adr.yaml
name: adr
entity: adr

frontmatter:
  type: object
  required: [title, status, date, discussed_in]
  properties:
    title: { type: string }
    status: { enum: [proposed, accepted, superseded] }
    date: { type: string, format: date }
    discussed_in:
      type: array
      items:
        type: string
        x-entity: [meeting-transcript, adr]

body:
  - rule: required-sections
    sections: [Context, Decision, Consequences]
```

```markdown
---
title: Use Postgres as primary store
status: accepted
date: 2026-04-20
discussed_in:
  - ../meetings/2026-04-15-architecture-sync.md # ✓ transcript
  - ../meetings/2026-04-22-postgres-deep-dive.md # ✓ transcript
  - ../rfcs/0001-multi-tenant.md # ✗ that's an RFC
  - ../meetings/2026-05-01-ghost.md # ✗ never existed
  - ../meetings/2026-04-15-architecture-sync.md#ok # ✗ has anchor
---
```

```
docs/adr/0001-use-postgres.md
  error[types.entity_ref] field 'discussed_in': link target '../rfcs/0001-multi-tenant.md': expected entity one of 'meeting-transcript', 'adr', got 'rfc'
  error[types.entity_ref] field 'discussed_in': link target '../meetings/2026-05-01-ghost.md' not found in workspace
  error[types.entity_ref] field 'discussed_in': link target '../meetings/2026-04-15-architecture-sync.md#ok' carries an anchor; entity references must be document-level
```

Three different failure classes, three message templates. The schema's `discussed_in` field is declared once — its shape (array of strings) and its edge type (one of `meeting-transcript` or `adr`) live next to each other.

## Out of scope

- **Id-based references.** v1 references are paths. Opaque ids resolved through a per-entity `id_field` would be rename-resilient, but they are a separate design.
- **Anchors inside reference values.** Rejected; section-level identity needs a different design.
- **`$ref` and composition support.** The walker is intentionally tiny and predictable in v1. Both can be added later by extending the walker without a contract change.
- **Backreferences / orphan detection.** Straightforwardly built later on the same entity index.
- **Body-link typing.** Body links carry no field name to attach a type expectation to; they remain under [`links.relative_path`](./rules.md#linksrelative_path) and [`links.obsidian_vault`](./rules.md#linksobsidian_vault).
