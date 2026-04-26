---
title: Index
---

## Outside table — sanity baseline

Aliased wikilink, no escape needed: [[Target|Alias]] resolves to Target.md.

## Inside a table — the regression case

| Topic                      | Link                             |
| -------------------------- | -------------------------------- |
| Plain target               | [[Target]]                       |
| Aliased target             | [[Target\|Alias]]                |
| Aliased target with anchor | [[Target#Section\|Custom Label]] |
| Missing target             | [[Nowhere\|Broken]]              |
