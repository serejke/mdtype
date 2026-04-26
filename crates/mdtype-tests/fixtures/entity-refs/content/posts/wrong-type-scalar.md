---
title: Wrong-type scalar post
author: ../tags/rust.md
---

## Body

`author:` is `x-entity: author` (single target), but the resolved file's entity is
`tag` — surfaces `target_type` (expected 'author', got 'tag').
