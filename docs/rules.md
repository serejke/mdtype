# Built-in Body Rules

Every rule shipped in `mdtype-rules-stdlib` is documented here. Each rule has a stable id (the value that appears on the `rule` field of every diagnostic) and a kebab-case shortform accepted in YAML. Rules are referenced from a schema's `body:` block.

## `body.forbid_h1`

YAML alias: `forbid-h1`. Parameters: none.

Disallows any top-level `# Heading` (level-1 ATX or setext) anywhere in the body. Use this when the file's title is supplied by frontmatter or by the filename and the body should only use `##` and below.

A diagnostic is emitted for every offending heading; the reported line is the absolute file line (frontmatter offset added).

```yaml
# schemas/blog-post.yaml
name: blog-post
body:
  - rule: forbid-h1
```

```markdown
---
title: Hello
---

# Stray H1 <-- triggers body.forbid_h1

## Summary

Body text.
```

## `body.required_sections`

YAML alias: `required-sections`. Parameters: `sections: [String]` (non-empty).

Asserts that each named heading appears in the body as a level-2 (`##`) heading. Matching is exact-text and case-sensitive; emphasis spans inside the heading are flattened (`## *Summary*` matches the section name `Summary`). Each missing section produces one whole-file diagnostic (no line) with an `AppendSection` fixit hint.

```yaml
# schemas/blog-post.yaml
body:
  - rule: required-sections
    sections: [Summary, Conclusion]
```

```markdown
---
title: Hello
---

## Summary

Intro.

(no `## Conclusion` --> triggers body.required_sections)
```

## `body.section_order`

YAML alias: `section-order`. Parameters: `order: [String]` (non-empty), `mode: strict | relaxed` (default `relaxed`).

Asserts that the listed sections appear as level-2 headings in the declared order. Both modes also flag any required section that is missing from the body.

- **`relaxed`** — required sections must appear in the listed order; other H2s may appear before, between, or after.
- **`strict`** — same ordering check, plus no other H2 may appear _between_ two consecutive required sections. Extras before the first or after the last required section are still allowed; use `body.forbidden_sections` if you want to ban them outright.

Each violation gets its own diagnostic with the offending heading's line.

```yaml
body:
  - rule: section-order
    order: [Summary, Background, Details, Conclusion]
    mode: relaxed
```

## `body.forbidden_sections`

YAML alias: `forbidden-sections`. Parameters: `sections: [String]` (non-empty).

Asserts that none of the named headings appear as a level-2 (`##`) heading. Useful for blocking scratchpad sections that should never reach a published file (e.g., `TODO`, `Scratch`, `WIP`). Each occurrence produces a diagnostic with the heading's line and a `DeleteLine` fixit hint.

```yaml
body:
  - rule: forbidden-sections
    sections: [TODO, Scratch, WIP]
```

```markdown
## Summary

ok

## TODO <-- triggers body.forbidden_sections

remember to remove this before shipping
```
