---
title: Index
---

## Resolutions

Basename match: [[Other]] resolves to content/Other.md.

Parent-suffix match: [[sub/Top]] picks the nested file.

Shortest-path tiebreak: [[Top]] resolves to the depth-2 file.

Alias rendering: [[Other|Friendly Name]] resolves to Other.md.

## Errors

Missing target: [[Nowhere]] does not exist.

Missing anchor: [[Other#No Such Heading]] picks the file but the
heading is absent.

Ambiguous at equal depth: [[Twin]] cannot be resolved.
