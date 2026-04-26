---
title: Wrong-type union post
author: ../authors/jane.md
related:
  - ../authors/john.md
---

## Body

`related:` is `x-entity: [post, tag]`. The value resolves to an `author` file —
none of the union alternatives match — surfaces `target_type`
(expected one of 'post', 'tag', got 'author').
