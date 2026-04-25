# Built-in Rules

Every rule shipped with `mdtype` is documented here. Each rule has a stable id (the value that appears on the `rule` field of every diagnostic) and is referenced from a schema:

- **Body rules** live in `mdtype-rules-stdlib` and run per-file under the `body:` block. They accept a kebab-case shortform in YAML alongside the canonical id.
- **Workspace rules** live in `mdtype-rules-stdlib` and `mdtype-rules-obsidian` and run after every file is parsed; they answer cross-file questions and go under the `workspace:` block. Workspace rules are canonical-id-only in v1.

## Body Rules

### `body.forbid_h1`

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

### `body.required_sections`

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

### `body.section_order`

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

### `body.forbidden_sections`

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

## Workspace Rules

Workspace rules answer cross-file questions: does this link resolve, is this basename ambiguous, do anchors point to real headings? They run after every file is parsed; the runner builds a `Workspace` index of files / headings / links / frontmatter, then each enabled rule judges its scope. Rules emit diagnostics only for files attached to their schema, but may freely _read_ facts about any file in the workspace (a link in one file may resolve to another file the rule never judges).

Each rule declares its required fact kinds. The runner unions those declarations across all enabled rules and configures the parser accordingly — for example, no rule requires `links_wiki` means comrak's wikilink extension stays off and `[[ ]]` is treated as ordinary text.

### `links.relative_path`

Crate: `mdtype-rules-stdlib`. Parameters: `ignore_schemes: [String]` (default `[http, https, mailto, tel]`), `check_anchors: bool` (default `true`).

Resolves inline Markdown links (`[text](path)`) against the source file's directory. The link is judged against the workspace, not the local filesystem: a target that exists on disk but was not part of the walked file set is reported as missing, matching the user's expectation that `mdtype <vault>` checks links _within_ that vault.

Anchors (`#fragment`) are matched against the target file's heading slugs (GitHub-flavored: lowercase, whitespace and underscores collapsed to `-`, punctuation dropped). A same-file anchor link (`[t](#section)`) checks against the source file's own headings. Schemes listed in `ignore_schemes` are skipped — typical case is external URLs you don't want flagged.

```yaml
workspace:
  - rule: links.relative_path
    check_anchors: true
    ignore_schemes: [http, https, mailto, tel]
```

```markdown
[the setup guide](setup.md) <-- resolves
[missing](nope.md) <-- reports 'not found in workspace'
[bad anchor](setup.md#nonexistent) <-- reports 'no heading matching anchor'
[external](https://example.com) <-- skipped (ignored scheme)
```

### `links.obsidian_vault`

Crate: `mdtype-rules-obsidian`. Parameters: `on_ambiguous: error | warn | first-match` (default `error`), `check_anchors: bool` (default `true`).

Resolves Obsidian-flavored wikilinks (`[[Target]]`, `[[Target|Alias]]`, `[[Target#Heading]]`) using Obsidian's policy:

1. **Exact path match** — if the target's parent components match a file's path tail (case-insensitive) and the basenames match, that file wins.
2. **Basename match** — among files sharing the target's basename, candidates are filtered by parent-component suffix when the target carries path components (e.g. `[[Folder/Note]]`).
3. **Shortest-path tiebreak** — among the surviving candidates, the file with the fewest path components wins. `Note.md` at the vault root beats `a/b/Note.md` deep inside.
4. **Equal-depth ambiguity** — if two-or-more candidates remain at the shortest depth, the rule reports a diagnostic listing every match. With `on_ambiguous: first-match`, the alphabetically-first candidate is silently picked.

Anchor matching uses the target file's raw heading text (Obsidian's convention), not the GitHub slug. `[[Note#Real Heading]]` matches a heading written exactly as `## Real Heading`.

```yaml
workspace:
  - rule: links.obsidian_vault
    on_ambiguous: error
    check_anchors: true
```

```markdown
[[Note]] <-- resolves to ./Note.md (shortest-path)
[[Folder/Sub/Note]] <-- exact-path match preferred
[[Twin]] <-- ambiguous when a/Twin.md and b/Twin.md exist
[[Note#Setup]] <-- checks against Note.md's '## Setup' heading
[[Note|Friendly Name]] <-- alias does not affect resolution
```

The two link rules are independent. Listing both in the same schema's `workspace:` block is supported: `links.relative_path` only judges inline `[t](path)` links, `links.obsidian_vault` only judges `[[ ]]` wikilinks, neither double-flags the same source.
