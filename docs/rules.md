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
