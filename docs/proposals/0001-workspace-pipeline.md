# Proposal 0001 — Workspace pipeline & cross-file rules

Status: **Implemented** (see [`CHANGELOG.md`](../../CHANGELOG.md), branch
`feat/workspace-pipeline`).
Scope: `mdtype-core`, `mdtype-rules-stdlib`, new crate `mdtype-rules-obsidian`, CLI runner.
Non-goal: any change to the JSON output schema's _shape_ — only new diagnostic `rule` ids.

This document is kept post-implementation as the design rationale. Behaviour, types, and
file paths it cites match the shipped code; the [Catalogue of rules](../rules.md) and
[Schema reference](../schema.md) are the user-facing docs. The [Worked examples](#15-worked-examples)
section at the bottom links to runnable scenarios under `examples/`.

## 1. Motivation

Today every check in `mdtype` is per-file. `BodyRule::check(&ParsedDocument, &mut Vec<Diagnostic>)`
in `crates/mdtype-core/src/rule.rs` and `Validator::validate(doc, schema)` in
`crates/mdtype-core/src/validator.rs` both take exactly one document. The CLI in
`crates/mdtype/src/main.rs` walks files, parses each, validates each, and moves on. A file
never sees another file.

This is sufficient for frontmatter validation, required sections, section order, and
forbid-h1 — every existing rule is local. It is **insufficient** for any check whose answer
depends on the rest of the file set:

- Does `[link](../other.md)` resolve to a file that exists?
- Does `[[Note Name]]` (Obsidian-style wikilink) resolve under the vault's chosen rules?
- Does `[[Note#Heading]]` reference a heading that actually exists in `Note`?
- Are there orphan notes (files no one links to)?
- Are there duplicate basenames whose `[[…]]` references are silently ambiguous?

These are _name resolution_ questions. Compilers split them out into a discovery pass that
builds a symbol table and a checking pass that consults it. mdtype needs the same split.

## 2. Current architecture (one phase)

```
walk files ─▶ for each file: parse ─▶ validate(doc, schema) ─▶ diagnostics
```

`Validator::validate` runs the JSON Schema frontmatter check, then each `BodyRule::check`
against the single document, then sorts and returns. There is no shared state across files
beyond the diagnostic accumulator.

## 3. Proposed pipeline (two phases)

```
walk files ─▶ parse all                              (existing)
            ─▶ collect: each rule contributes facts to Workspace
            ─▶ check:   each rule judges with &Workspace in scope
            ─▶ sort + report
```

- **Collect** produces _facts_. No diagnostics. Just an index.
- **Check** produces _judgments_. Reads facts, emits diagnostics.

Splitting is mandatory because any cross-file judgment about file _A_ may depend on facts
from a file _B_ that is encountered later in the walk. Both passes iterate the same parsed
documents; phase 1 has to finish before phase 2 starts.

Per-file rules (`BodyRule`) keep their current trait and run unchanged in the check phase
— they simply ignore the workspace.

## 4. New types in `mdtype-core`

### 4.1 `Workspace`

A neutral, policy-free index of facts collected from every parsed document. Owned by core,
populated by rules during the collect phase, read by rules during the check phase.

```rust
pub struct Workspace {
    /// Every parsed file's path, in walk order. Source of truth for "what files exist."
    pub files: Vec<PathBuf>,
    /// basename without extension → all files with that basename. Powers Obsidian-style
    /// shortest-path resolution.
    pub by_basename: HashMap<String, Vec<PathBuf>>,
    /// path → headings emitted by that file (text + slug + line).
    pub headings: HashMap<PathBuf, Vec<HeadingFact>>,
    /// path → links emitted by that file.
    pub links: HashMap<PathBuf, Vec<LinkRef>>,
    /// path → frontmatter JSON. Already available; surfaced here so workspace rules don't
    /// need to re-borrow ParsedDocument.
    pub frontmatter: HashMap<PathBuf, serde_json::Value>,
}
```

The Workspace is **policy-free**: it stores facts, never opinions about what those facts
mean. `by_basename` does not pick a winner for ambiguous names — that is the resolver's
job, in the check phase.

### 4.2 `LinkRef`

```rust
pub struct LinkRef {
    pub kind: LinkKind,        // Inline, Reference, Wiki, WikiEmbed
    pub target: String,        // the destination as comrak surfaces it (see note)
    pub anchor: Option<String>,// fragment after `#`
    pub alias: Option<String>, // wikilink alias (`[[Target|Alias]]`)
    pub line: usize,           // 1-indexed, absolute (frontmatter offset already applied)
}
```

`target` is what the resolver consumes. For inline `[t](path)` it is the URL after
comrak's normalization (`comrak::nodes::NodeLink::url`). For wikilinks
(`comrak::nodes::NodeWikiLink::url`) comrak applies a `clean_url` pass that does
HTML-entity decoding and trims whitespace; the field stores the post-cleanup string. For
`[[A|B]]`, comrak puts the alias as child text of the wikilink node, which the extractor
captures into `alias` while taking `target = "A"` from the node URL.

This is **lossy by design**: a future rule that needs the exact source bytes (e.g. to
diagnose unusual escaping) would need an extension that records source spans alongside
the link. v1 does not. The Obsidian and relative-path resolvers we ship operate on
post-cleanup strings without issue because their resolution semantics (basename match,
relative path normalization) align with the cleanup pass.

### 4.3 `HeadingFact`

```rust
pub struct HeadingFact {
    pub text: String,
    pub slug: String,   // GitHub-style slug, used for `#anchor` matching
    pub level: u8,
    pub line: usize,
}
```

### 4.4 `WorkspaceRule`

A sibling trait to `BodyRule`. Every implementation declares what facts it needs; core
populates the workspace from those declarations and then calls `check`. Rules are pure
judges — they do not contribute to `Workspace` directly.

```rust
pub trait WorkspaceRule: Send + Sync {
    fn id(&self) -> &'static str;

    /// What this rule needs the collect phase to gather. Core unions all rules'
    /// requirements across all schemas in the run, then runs the corresponding extractors
    /// against every parsed file. Same `Requirements` value also drives parser flags.
    fn requires(&self) -> Requirements { Requirements::default() }

    /// Read facts from `ws`, emit diagnostics.
    ///
    /// `scope` is the slice of `ws.files` for which this rule instance is enabled —
    /// i.e. the files whose attached schema lists this exact rule entry. The rule must
    /// only emit diagnostics for files inside `scope`; it may freely *read* any fact in
    /// `ws` (a wikilink in a scope file may resolve to an out-of-scope file, and the
    /// out-of-scope file's headings are still indexed because facts are gathered
    /// globally, not per-scope).
    fn check(&self, ws: &Workspace, scope: &[PathBuf], out: &mut Vec<Diagnostic>);
}

#[derive(Default, Clone, Copy)]
pub struct Requirements {
    pub headings: bool,
    pub links_inline: bool,
    pub links_wiki: bool,
}
```

Each rule **instance** is built per schema entry and carries its own params, so two
schemas enabling `links.obsidian_vault` with different `on_ambiguous` settings produce two
independent rule objects with two independent `scope` slices. Different parameterizations
across schemas are therefore handled by ordinary instance state — the runner does not have
to merge them.

**Why core owns extraction (no per-rule `collect`).** `Workspace` facts are needed by
_every_ enabled rule, including for files outside any individual rule's `scope` (a link
from in-scope file `A` may target out-of-scope file `B`, and `B`'s headings still need to
be indexed for `check_anchors`). If extraction were per-rule, a rule scoped to `A` would
either miss `B`'s headings or have to re-run scan logic on every parsed file regardless
of scope — at which point "per-rule" buys nothing. So core ships canonical extractors
keyed by `Requirements` flags:

| Flag           | Extractor populates                                                                                          |
| -------------- | ------------------------------------------------------------------------------------------------------------ |
| `headings`     | `Workspace::headings[path]`                                                                                  |
| `links_inline` | `Workspace::links[path]` with `LinkKind::{Inline,Reference}` entries                                         |
| `links_wiki`   | `Workspace::links[path]` with `LinkKind::{Wiki,WikiEmbed}` entries; also enables comrak's wikilink extension |

A future rule that needs a fact type not in this table must extend `Requirements` and
ship the corresponding extractor in `mdtype-core`. This is the trait-boundary cost of
keeping rules pure.

`Requirements` doubles as the parser-flag driver: if no rule requires `links_wiki`, the
comrak wikilinks extension stays off and `[[ … ]]` is treated as ordinary text.

### 4.5 Schema additions

```rust
pub struct Schema {
    pub name: String,
    pub description: Option<String>,
    pub frontmatter: Option<serde_json::Value>,
    pub body: Vec<Box<dyn BodyRule>>,
    pub workspace: Vec<Box<dyn WorkspaceRule>>, // NEW
}

impl Default for Schema { /* all fields empty / None / Vec::new() */ }
```

Adding a public field is a **source-level break** for any caller that builds a `Schema`
with a struct literal — every such site must add `workspace: Vec::new()` (or use
`..Schema::default()`). In-tree this affects:

- `crates/mdtype-core/src/validator.rs` (two test literals).
- `crates/mdtype-schema-yaml/src/lib.rs:141` (the loader's final `Schema { … }`).
- Any future fixture or test that constructs a `Schema` directly.

The migration ships in the same commit as the trait. Downstream rule crates that depend on
`mdtype-core` and construct schemas in tests will need the same one-line addition; this is
called out in `CHANGELOG.md` as a 0.x source-level break, justified by the
trait-boundary invariant (no alternative keeps `Schema` extensible without breaking
literals). `Default` mitigates future additions: subsequent fields can be added as
`..Schema::default()` patterns with no further break, provided every new field has a
sensible `Default`.

A `#[non_exhaustive]` alternative was considered and rejected: it would force every
constructor in the codebase through a builder, which is a bigger churn for less benefit
than adding one field with a `Default`.

### 4.6 `WorkspaceRuleFactory`

Mirrors `BodyRuleFactory`. Registered alongside body factories in
`mdtype-schema-yaml`'s factory registry. The YAML loader picks the right trait based on the
factory found for a given rule id.

## 5. Pipeline change in `CoreValidator`

The single `validate(doc, schema)` call site in the CLI becomes a multi-phase driver.
**Parsing moves into the runner** — it must, because comrak parser flags depend on the
union of `Requirements` declared by enabled workspace rules, which can only be computed
once schemas are known. The runner takes already-resolved per-file schemas (the CLI keeps
ownership of `Mode`, factories, and the override cache; see §6), owns the `Arena`, picks
parser flags, extracts facts, and runs checks.

To avoid borrow-checker contortions during override loading (the CLI grows its
`Vec<Schema>` while pre-passing files, and a `Vec` reallocation would invalidate any
held `&Schema`), the runner takes a slice of schemas **plus** a parallel slice of
indices, not refs:

```rust
pub fn run_workspace(
    files: &[PathBuf],            // only files that passed the CLI's frontmatter pre-pass
    schemas: &[Schema],           // CLI-owned pool, includes glob-map + override schemas
    schema_idx: &[Option<usize>], // schema_idx[i] = index into `schemas` for files[i],
                                  //   None = no glob/override match (file still indexed)
) -> Result<Vec<Diagnostic>, Error> {
    // 1. Build the rule-instance roster: a flat list of (rule, scope) pairs, one entry
    //    per workspace-rule entry across all schemas referenced by `schema_idx`. `scope`
    //    is the subset of `files` whose schema contains that rule entry (preserves rule-
    //    instance identity across schemas: two schemas listing the same id produce two
    //    entries with different scopes and possibly different params).
    // 2. Union Requirements across every (rule, _) in the roster.
    //    Translate to `comrak::Options`. As of comrak 0.28, wikilink syntax is enabled
    //    via `extension.wikilinks_title_after_pipe` (Obsidian-flavored
    //    `[[Target|Alias]]`, alias after the pipe). `wikilinks_title_before_pipe` exists
    //    for the inverted convention; the runner enables `_after_pipe` for `links_wiki`
    //    and only flips to `_before_pipe` if a future rule requests it via an extended
    //    `Requirements` flag.
    // 3. Allocate one Arena for the whole run; parse every file with the unioned options
    //    via a new options-aware parse helper (see "Parse API" below). Body-parse
    //    failures emit PARSE_RULE_ID diagnostics — these files are skipped for fact
    //    extraction and rule execution but remain in `ws.files` so other files can still
    //    reference their path.
    // 4. Build Workspace: always populate `files` and `by_basename`. For each
    //    Requirements flag set in the union, run the corresponding core extractor
    //    against every parsed file (not scoped — facts are global; see §4.4).
    // 5. Per-file body rules: current `BodyRule::check` semantics, unchanged.
    // 6. Workspace rules: for each (rule, scope) in the roster, call
    //    `rule.check(&ws, scope, &mut diags)`. The rule emits diagnostics only for files
    //    in `scope` but may read facts about any file in `ws`.
    // 7. Sort by (file, line, rule).
}
```

Per-file body rule semantics are unchanged; step 5 is exactly what `CoreValidator::validate`
does today, hoisted into the workspace driver. The existing `Validator` trait stays for
downstream consumers who validate one document at a time.

**Parse API.** `parse_file(path, arena)` in `crates/mdtype-core/src/parser.rs:42` today
hard-codes `comrak::Options::default()`. The runner needs an options-aware variant.
Cleanest additive change:

```rust
pub fn parse_file_with_options<'a>(
    path: &Path,
    arena: &'a Arena<AstNode<'a>>,
    options: &comrak::Options,
) -> Result<ParsedDocument<'a>, Error>;

// Existing function becomes a thin wrapper, preserving the current signature for
// downstream consumers that don't care about parser flags:
pub fn parse_file<'a>(path: &Path, arena: &'a Arena<AstNode<'a>>) -> Result<ParsedDocument<'a>, Error> {
    parse_file_with_options(path, arena, &comrak::Options::default())
}
```

This is purely additive — no source-level break for existing callers.

## 6. CLI changes

The `run()` function in `crates/mdtype/src/main.rs` currently parses-and-validates each file
in a single loop pass. It splits into:

1. Walk files and build the schema roster (existing `Mode` / glob-set logic, unchanged).
2. **Frontmatter-only pre-pass**: for each file, the CLI calls a new public helper
   `mdtype_core::parser::split_frontmatter` (today this function is private inside
   `parse_file`; the change exposes it). It returns the parsed frontmatter without
   running comrak. With that frontmatter the CLI runs the existing
   `resolve_schema_index` (`crates/mdtype/src/main.rs:256`) — including the
   `frontmatter.schema:` override path — and grows the CLI's `schemas: Vec<Schema>`
   accordingly. Output of the pre-pass is two parallel vectors: `Vec<PathBuf>` of files
   that pre-passed cleanly, and `Vec<Option<usize>>` of their resolved schema indices.
   `Mode`, factories, and override cache stay in the CLI exactly as they live today.
3. **Pre-pass failures** (frontmatter unreadable or malformed) are accumulated into a
   CLI-side `Vec<Diagnostic>` of `PARSE_RULE_ID` entries and **excluded** from the
   `files` slice passed to the runner. This means the runner never re-attempts a
   frontmatter parse the CLI already failed, eliminating any risk of duplicate parse
   diagnostics. The runner's own `PARSE_RULE_ID` emissions cover only body-parse
   failures (which can only happen for files whose frontmatter already parsed).
4. Hand `(files, schemas, schema_idx)` to `run_workspace`. The runner owns the arena and
   the body-parsing pass — the CLI no longer calls `parse_file` directly.
5. Concatenate pre-pass diagnostics + runner diagnostics, sort, then **recompute
   `files_with_errors`** from the merged diagnostic list before building `Summary`.
   Today the per-file loop incrementally tracks this set; that increment site disappears,
   so a final pass over the diagnostic vector takes its place.

No new CLI flags. No change to `--config`, `--schema`, `--format`, `--quiet`, `--no-color`.

Arena lifetime: today the CLI creates a fresh arena per file. The new runner allocates one
arena for the whole run so every `ParsedDocument` outlives the collect phase. This is an
internal change, invisible to any caller.

The frontmatter is parsed twice per file (once in the CLI pre-pass, once again inside
`parse_file_with_options` during the runner's body parse). YAML frontmatter is small and
the cost is negligible compared to comrak; deduplicating would require a multi-stage parse
API in `mdtype-core` that is not worth the surface-area expansion for v1.

## 7. First two workspace rules

Both ship with stable rule ids and full docs in `docs/rules.md`.

### 7.1 `links.relative_path` (in `mdtype-rules-stdlib`)

Resolves inline / reference Markdown links (`[t](path.md)`, `[t][id]`) as paths relative to
the source file's directory. Rejects the link if:

- the target file does not exist in the workspace,
- the target file exists but `#anchor` does not match any heading slug in it,
- the target string contains an unsupported scheme other than empty, `http(s)`, `mailto:`,
  `tel:` (configurable via rule params).

Wikilinks are ignored by this rule (different kind).

```yaml
- rule: links.relative_path
  ignore_schemes: [http, https, mailto, tel] # default
  check_anchors: true # default
```

### 7.2 `links.obsidian_vault` (in new crate `mdtype-rules-obsidian`)

Resolves `[[Note]]`, `[[Folder/Note]]`, `[[Note|Alias]]`, `[[Note#Heading]]`, and
optionally `![[Note]]` embeds, using Obsidian's resolution rules:

1. Exact path match against any file in the workspace.
2. Else basename match: pick the file whose path has the fewest segments.
3. Tie at equal depth → diagnostic by default (configurable).

```yaml
- rule: links.obsidian_vault
  on_ambiguous: error # error | warn | first-match
  check_anchors: true # validate `#Heading` against the target's headings
```

Embed syntax (`![[Note]]`) is out of scope for v1; see §11.

### 7.3 Why two rules and not one

Inline-link semantics and wikilink semantics are independent axes (see §11). A project may
enable either, both, or neither. mdtype core ships zero opinions about link semantics; the
choice is made by which rule(s) the schema enables. This matches the existing rule model
(`body.required_sections` and `body.section_order` are also independent).

## 8. Schema YAML

```yaml
# crates/mdtype-tests/fixtures/links_obsidian/schemas/note.yaml
name: vault-note
description: Obsidian-style notes
body: []
workspace:
  - rule: links.obsidian_vault
    on_ambiguous: error
    check_anchors: true
```

A project can mix:

```yaml
workspace:
  - rule: links.relative_path
  - rule: links.obsidian_vault
    on_ambiguous: error
```

Each rule processes only the link kinds it cares about. Inline `[t](foo.md)` is judged by
`links.relative_path`; `[[Foo]]` is judged by `links.obsidian_vault`. They never conflict.

Rule ids in YAML use the canonical form (matching what appears in the diagnostic `rule`
field). Body rules accept a kebab shortform — `forbid-h1` resolves to `body.forbid_h1`
because the loader strips a `body.` prefix at
`crates/mdtype-schema-yaml/src/lib.rs:199`. Whether workspace rules get an analogous
shortform is left as an open question (§10); v1 ships them canonical-only.

## 9. JSON contract impact

`docs/json-schema.md` is a versioned public contract. This proposal is **additive**:

- `Diagnostic` shape is unchanged.
- New `rule` ids appear: `links.relative_path`, `links.obsidian_vault`. The `rule` field is
  documented as an opaque string; new ids are not a breaking change.
- No new `Fixit` variants required for the first cut. (`Fixit::Custom` already absorbs any
  rule-specific hint.)

`version` does not bump.

## 10. Open questions

1. **Workspace memory cost.** A vault with thousands of notes will hold every AST in memory
   for the duration of the run. Acceptable for v1 (mdtype is invoked on a project, not a
   monorepo of vaults). If it becomes an issue: store only the facts in `Workspace`, drop
   the AST after collect. This is purely an internal change, no contract impact.

2. **Heading slug algorithm.** GitHub-flavored slugification is the de facto default but
   Obsidian uses raw heading text. Resolution: store `text` and `slug` in `HeadingFact`;
   each link rule decides which to match against. `links.relative_path` matches `slug`,
   `links.obsidian_vault` matches `text`.

3. **Files outside the walk root.** A wikilink may target a file that exists on disk but
   wasn't included in `cli.paths`. Resolution: workspace contains exactly the walked file
   set. A link to a file outside the walk is a `NotFound` diagnostic. Expanding the walk is
   the user's responsibility (same as today).

4. **Workspace-rule kebab shortform.** Body rules accept `forbid-h1` as a shortform for the
   canonical id `body.forbid_h1` because the loader strips a `body.` prefix and rewrites
   `_` to `-`. We can extend the same trick to workspace rules (strip `links.`, etc.) or
   ship workspace rules as canonical-only. The first option is more uniform but requires
   the loader to know each namespace prefix; the second is simpler. Default proposal: keep
   workspace rules canonical-only for v1 and revisit if YAML noise becomes a complaint.

## 11. Non-goals

- Autofix. mdtype still never rewrites files. `Fixit` hints may carry suggestions; no
  rewriter ships.
- URL rewriting (`.md` → `/blog/foo`). Build-step concern, out of scope.
- Cross-vault resolution. A run scopes to one walk root.
- Custom resolvers via plugin DLLs. New resolvers ship as new crates that depend on
  `mdtype-core`, same as `mdtype-rules-obsidian`.
- Bumping the JSON schema `version`. This is purely additive.
- Wikilink embeds (`![[Note]]`). The extractor surfaces only `LinkKind::Wiki`; embed
  syntax would need a new `LinkKind` variant and an extractor change.

## 12. Testing

Every fixture under `crates/mdtype-tests/fixtures/` snapshots both reporters; the
shipped scenarios follow the same convention:

- [`links-relative-path/`](../../crates/mdtype-tests/fixtures/links-relative-path/) —
  valid relative link, broken relative link, anchor match, broken anchor, external
  scheme (ignored), same-file anchor.
- [`links-obsidian-vault/`](../../crates/mdtype-tests/fixtures/links-obsidian-vault/) —
  basename match, parent-suffix match, shortest-path tiebreak, equal-depth ambiguity,
  missing target, missing anchor, alias rendering.
- [`links-mixed/`](../../crates/mdtype-tests/fixtures/links-mixed/) — both rules
  enabled on the same vault, asserting they don't double-flag.

Snapshot regeneration: `UPDATE_FIXTURES=1 cargo test -p mdtype-tests --test fixtures`.

## 13. Migration & backwards compatibility

- `BodyRule` trait is unchanged. All four stdlib body rules keep working.
- `Schema` gains a `workspace: Vec<Box<dyn WorkspaceRule>>` field; existing schemas
  leave it empty. `Schema` now derives `Default` so downstream callers can use
  `..Schema::default()` to absorb future fields without further source breaks.
- `Validator` trait is unchanged; `CoreValidator::validate` retains its single-document
  signature for downstream consumers who don't want the workspace pipeline.
- `load_schema_file` and `YamlSchemaSource::new` take both factory registries
  (`BodyRuleFactory`, `WorkspaceRuleFactory`). Pass an empty `Arc::new(Vec::new())` for
  the kind you don't extend.
- The CLI binary's behavior is unchanged for any project whose schemas have no
  `workspace:` block.

## 14. Out-of-scope follow-ups (sketch only)

The workspace pipeline makes these straightforward additions; none need core changes:

- `links.orphans` — files no other file links to.
- `headings.unique` — duplicate slugs across the workspace.
- `frontmatter.unique` — duplicate ids in frontmatter (e.g. `slug:`).
- `links.bidirectional` — for vaults that require backlinks.

The fourth extension example in [`docs/extending.md`](../extending.md) walks through
implementing one of these (`frontmatter.unique_id`) end to end.

## 15. Worked examples

Two runnable example projects ship under `examples/`:

- [`examples/blog-site/`](../../examples/blog-site/) — `links.relative_path` enabled on
  blog and doc schemas; the new
  [`2026-04-cross-references.md`](../../examples/blog-site/content/posts/2026-04-cross-references.md)
  post demonstrates valid sibling and cross-folder links plus an anchor reaching into a
  doc page.
- [`examples/wiki-vault/`](../../examples/wiki-vault/) — minimal Obsidian-flavored vault
  for `links.obsidian_vault`. Demonstrates basename match, shortest-path tiebreak,
  parent-suffix disambiguation, raw-heading-text anchor matching, and alias rendering.
  The example's [README](../../examples/wiki-vault/README.md) walks the user through
  deliberate edits to surface each diagnostic class.
