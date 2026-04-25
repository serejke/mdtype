# Example: wiki-vault

A small Obsidian-flavored vault demonstrating
[`links.obsidian_vault`](../../docs/rules.md#linksobsidianvault).

```
notes/
  Index.md            <- entry point with wikilinks to siblings and a sub-note
  Topic.md            <- exact-path target
  Subtopic.md         <- shortest-path winner; Index links via [[Subtopic]]
  archive/
    Subtopic.md       <- same basename as the root-level Subtopic.md, deeper
```

`Subtopic` resolves to `notes/Subtopic.md` (shortest path). Targeting the
archived note specifically requires `[[archive/Subtopic]]`.

## Run it

```sh
mdtype examples/wiki-vault
```

Every wikilink in `Index.md` resolves; the example exits `0`.

## Tweak it to see diagnostics

- Add `[[Nowhere]]` to `Index.md` → `wikilink target 'Nowhere' not found in workspace`.
- Rename `notes/Subtopic.md` to `notes/topics/Subtopic.md` and create
  `notes/other/Subtopic.md` → `[[Subtopic]]` becomes ambiguous and the rule
  reports both candidates.
- Add `[[Topic#Missing Heading]]` to `Index.md` → reports a missing anchor
  using Obsidian's raw-heading-text match.
