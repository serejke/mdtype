//! Cross-file index and the data types workspace rules consume.
//!
//! A [`Workspace`] is built once per run by `mdtype-core`'s extractors. It is policy-free:
//! it stores facts about every parsed file (paths, headings, link references, frontmatter)
//! and never picks a winner among ambiguous lookups. Resolution policy lives entirely in
//! the rules that consume it.

use std::collections::HashMap;
use std::path::PathBuf;

/// What kinds of facts a [`crate::WorkspaceRule`] needs `mdtype-core` to extract.
///
/// The runner unions every enabled rule's requirements across all schemas in a run, then
/// runs the corresponding extractors against every parsed file. The same union also
/// drives parser-flag selection: e.g. `links_wiki` flips comrak's
/// `extension.wikilinks_title_after_pipe`.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Requirements {
    /// Populate [`Workspace::headings`] for every parsed file.
    pub headings: bool,
    /// Populate [`Workspace::links`] with `LinkKind::Inline` and `LinkKind::Reference` entries.
    pub links_inline: bool,
    /// Populate [`Workspace::links`] with `LinkKind::Wiki` and `LinkKind::WikiEmbed`
    /// entries. Also enables comrak's wikilink extension during parsing.
    pub links_wiki: bool,
}

impl Requirements {
    /// Field-wise OR. Used by the runner to union requirements across rule instances.
    #[must_use]
    pub const fn merge(self, other: Self) -> Self {
        Self {
            headings: self.headings || other.headings,
            links_inline: self.links_inline || other.links_inline,
            links_wiki: self.links_wiki || other.links_wiki,
        }
    }
}

/// What kind of source syntax produced a [`LinkRef`].
///
/// Reference-style links (`[text][label]`) collapse into [`LinkKind::Inline`] because
/// comrak resolves the label at parse time and the AST loses the distinction. Embeds
/// (`![[Target]]`) are not surfaced in v1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkKind {
    /// `[text](destination)` and its reference-link equivalents post-resolution.
    Inline,
    /// `[[Target]]` or `[[Target|Alias]]` (Obsidian-flavored wikilinks).
    Wiki,
}

/// One link occurrence found in a parsed file.
#[derive(Clone, Debug)]
pub struct LinkRef {
    /// Which source syntax produced this entry.
    pub kind: LinkKind,
    /// Destination as comrak surfaces it (post normalization / `clean_url`). For
    /// wikilinks, this is the target portion before the optional `|alias`. Lossy by
    /// design — a future rule that needs raw source bytes must extend the extractor.
    pub target: String,
    /// Fragment after `#`, if any. Stripped from `target` so resolvers don't have to
    /// re-split.
    pub anchor: Option<String>,
    /// Wikilink alias (`[[Target|Alias]]` → `Some("Alias")`). `None` for inline /
    /// reference links.
    pub alias: Option<String>,
    /// 1-indexed line in the original source (frontmatter offset already applied).
    pub line: usize,
}

/// One heading occurrence found in a parsed file.
#[derive(Clone, Debug)]
pub struct HeadingFact {
    /// Rendered heading text — child inlines flattened, formatting stripped.
    /// Matched by Obsidian-style `[[Target#Heading]]` resolvers.
    pub text: String,
    /// GitHub-flavored slug derived from `text`. Matched by Markdown anchor
    /// resolvers (`[t](file.md#heading)`).
    pub slug: String,
    /// 1 through 6.
    pub level: u8,
    /// 1-indexed line in the original source (frontmatter offset already applied).
    pub line: usize,
}

/// Cross-file index assembled by the runner before any rule is judged.
///
/// **Policy-free**: lookups like [`by_basename`](Self::by_basename) return every match
/// without picking a winner. The rule's resolver decides what "best match" means.
#[derive(Default, Debug)]
pub struct Workspace {
    /// Every parsed file's path, in walk order. Source of truth for "what files exist."
    /// Files whose body parse failed are absent.
    pub files: Vec<PathBuf>,
    /// Lower-cased basename without extension → all files with that basename, in walk
    /// order. Powers Obsidian-style shortest-path resolution.
    pub by_basename: HashMap<String, Vec<PathBuf>>,
    /// Path → headings emitted by that file. Empty for files with no headings or for
    /// runs whose rules did not request `Requirements::headings`.
    pub headings: HashMap<PathBuf, Vec<HeadingFact>>,
    /// Path → links emitted by that file. Empty for runs whose rules did not request
    /// any `links_*` fact kind.
    pub links: HashMap<PathBuf, Vec<LinkRef>>,
    /// Path → frontmatter JSON for every successfully pre-passed file. `Null` for files
    /// without a frontmatter block.
    pub frontmatter: HashMap<PathBuf, serde_json::Value>,
    /// Path → entity name for every file whose resolved schema declared `entity:`.
    /// Files matched by no schema, or by a schema without `entity:`, are absent from the
    /// map. See `docs/types.md`.
    pub entities: HashMap<PathBuf, String>,
}
