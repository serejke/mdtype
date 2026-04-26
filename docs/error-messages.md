# Error Message Style Guide

Every diagnostic emitted by `mdtype` (or by a downstream rule) is consumed by humans **and** by LLM agents driving fixes. The JSON contract already gives `rule`, `file`, `line`, `severity`, and an optional `fixit`; the `message` field on top should be terse, declarative, and unambiguous about what to change.

## Rules

1. **Imperative voice.** Say what is wrong, not "this seems to". `top-level H1 is not allowed`, not `should not contain`.
2. **Mention the offending value verbatim**, quoted with single quotes: `forbidden section 'TODO'`. Lets an agent locate the token in source without scraping context.
3. **Mention the expected shape** when the failure is a type/structure mismatch: `expected array, found "single"`. Pair the actual and expected so the fix is obvious.
4. **Lead with the failure class** when the problem is at parse/IO level: `frontmatter parse failed: missing closing '---' fence on line 5`. The first colon-separated chunk is the category, the rest is the detail.
5. **One sentence, no parenthetical aside.** Anything an agent needs after the period belongs on `fixit`, not in the message.
6. **No Markdown.** No backticks, no asterisks, no headings. Quotes only.
7. **No file paths in the message.** `file` and `line` are separate fields; embedding the path again is duplication.
8. **No prose explanation of the rule.** That belongs in [`docs/rules.md`](./rules.md). The message is a single concrete violation.
9. **Stable wording.** Treat the message text as best-effort human-readable, but do not gratuitously rewrite it once shipped — agents may build prompts around current phrasing. Snapshot tests in `crates/mdtype-tests/fixtures/` lock the wording per rule.

## Templates

| Failure class                     | Template                                                                                            |
| --------------------------------- | --------------------------------------------------------------------------------------------------- |
| Missing required field            | `missing required field '{name}' (expected {type})`                                                 |
| Type mismatch                     | `field '{name}': expected {type}, found {value}`                                                    |
| Disallowed extra field            | `unexpected field '{name}' (schema declares additionalProperties=false)`                            |
| Disallowed top-level construct    | `top-level {construct} '{text}' is not allowed`                                                     |
| Missing required section          | `missing H{level} section '{name}'`                                                                 |
| Section out of order              | `H{level} section '{name}' is out of order; should appear before '{prev}'`                          |
| Section not allowed by policy     | `H{level} section '{name}' is not allowed (forbidden by schema)`                                    |
| Reference field has wrong shape   | `field '{field}': expected string or array of strings, found {kind}`                                |
| Reference value carries an anchor | `field '{field}': link target '{path}' carries an anchor; entity references must be document-level` |
| Reference target missing          | `field '{field}': link target '{path}' not found in workspace`                                      |
| Reference target has no entity    | `field '{field}': link target '{path}' has no declared entity, expected {expected}`                 |
| Reference target has wrong entity | `field '{field}': link target '{path}': expected entity {expected}, got '{actual}'`                 |
| Parse / IO failure                | `{phase} failed: {detail}`                                                                          |

`{expected}` is `'NAME'` for a single target and `one of 'A', 'B'` for a union of entity types.

## Anti-patterns

- ❌ `"author" is a required property` (unclear, no expected type, mid-sentence quoting).
- ❌ `null is not of type "object"` (technically accurate, useless to a fixer — the actual problem is "no frontmatter block").
- ❌ `H1 should not be at top level` (passive, no offending text quoted).
- ❌ `Heading 'TODO' is forbidden by the schema (consider removing it or moving the content to a different section)` (parenthetical aside, prescriptive prose).

## Where to look

- `frontmatter.schema` is emitted from [`crates/mdtype-core/src/validator.rs`](../crates/mdtype-core/src/validator.rs).
- `mdtype.parse` is emitted from [`crates/mdtype/src/main.rs`](../crates/mdtype/src/main.rs).
- All `body.*` rules live under [`crates/mdtype-rules-stdlib/src/`](../crates/mdtype-rules-stdlib/src/).
- Snapshot pinning lives under [`crates/mdtype-tests/fixtures/<scenario>/expected/`](../crates/mdtype-tests/fixtures/).
