---
title: A post with a broken author profile reference
date: 2026-06-01
tags: [meta]
author: Jane Roe
author_profile: ../authors/ghost.md
---

## Summary

`author_profile:` points at a path that does not exist in the workspace, so the
runtime reports `target_missing` under the `types.entity_ref` diagnostic id.

## Background

Pair this post with `2026-05-typed-author.md` to see one happy path and one
failure side-by-side. Both posts share the same schema; only the value of
`author_profile:` differs.

## Details

The diagnostic carries no `line` (frontmatter values have no per-element source
location in v1); the offending field name is part of the message so the value
is locatable in the YAML block.

## Conclusion

This file deliberately violates the schema; do not copy it as a template.
