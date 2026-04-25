# `examples/multi-folder/` — schema-per-folder

Demonstrates the recommended layout when one project mixes note types: **one root config, multiple globs, each pointing at its own schema file**. Distilled from a real Obsidian-vault dogfood (`Daily/` notes have a recurring `## Notes` section that the default schema would flag; `CLAUDE.md` folder-entry notes have no frontmatter at all).

## Layout

```
multi-folder/
├── .mdtype.yaml          # one root config; three globs in priority order
├── schemas/
│   ├── note.yaml         # default knowledge note: created/updated/description, no scratch
│   ├── daily-note.yaml   # daily journal: same frontmatter, but ## Notes is allowed
│   └── folder-entry.yaml # CLAUDE.md folder-entry: frontmatter optional, no scratch
└── content/
    ├── posts/intro.md
    ├── daily/2026-04-25.md
    └── team/CLAUDE.md
```

## Run

```
mdtype examples/multi-folder/
```

Exits `0` — every fixture matches its glob and conforms to its schema.

## Glob resolution

`mdtype` matches each `.md` file against the `rules:` list in declaration order and uses the **first** glob that hits. Order matters — most specific first, catch-all last. The config above:

1. `content/daily/**/*.md` → `daily-note.yaml`
2. `content/team/**/CLAUDE.md` → `folder-entry.yaml`
3. `content/**/*.md` → `note.yaml`

## Config-file resolution

`mdtype` walks **up** from the cwd (and falls back to the input path's parent) and uses the first `.mdtype.yaml` it finds. There is **no merging** of parent and child configs — the closest one wins, period. If you want different rules for different folders inside one project, use globs like the example above. Don't sprinkle `.mdtype.yaml` files at every level.
