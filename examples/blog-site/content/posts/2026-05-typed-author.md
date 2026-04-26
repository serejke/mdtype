---
title: A post with a typed author profile reference
date: 2026-05-12
tags: [meta]
author: Jane Roe
author_profile: ../authors/jane-roe.md
---

## Summary

This post sets `author_profile:` to a relative path. The schema declares
`x-entity: author` on that property, so the runtime walks the path against the
workspace and checks that the target file's declared entity is `author`.

## Background

A wrong target (a doc page, a missing file, a typed-but-different entity) would
surface a `types.entity_ref` diagnostic. See `2026-06-broken-references.md` for
a deliberate failure case.

## Details

The reference is path-based: the value is a relative path, resolved against the
source file's directory, and the resolved file's entity (set by its schema's
`entity:` field) is checked against the expected entity from the annotation.

## Conclusion

Typed references make a Markdown knowledge base type-check end-to-end.
