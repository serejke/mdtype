//! `links.obsidian_vault` — resolve `[[wikilinks]]` using Obsidian's policy:
//! exact-path → basename → shortest-path. Ambiguities at equal depth are surfaced as
//! diagnostics by default.

use std::path::{Path, PathBuf};

use mdtype_core::{
    Diagnostic, Error, HeadingFact, LinkKind, LinkRef, Requirements, Severity, Workspace,
    WorkspaceRule, WorkspaceRuleFactory,
};
use serde::Deserialize;

/// Stable rule id, exposed for downstream crates to reference.
pub const ID: &str = "links.obsidian_vault";

/// Wikilink integrity check.
pub struct Rule {
    on_ambiguous: AmbiguityPolicy,
    check_anchors: bool,
}

/// What to do when a wikilink resolves to two-or-more files at the same depth.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AmbiguityPolicy {
    /// Emit `Severity::Error` (default).
    Error,
    /// Emit `Severity::Warning`.
    Warn,
    /// Pick the alphabetically-first match silently.
    FirstMatch,
}

impl WorkspaceRule for Rule {
    fn id(&self) -> &'static str {
        ID
    }

    fn requires(&self) -> Requirements {
        Requirements {
            headings: self.check_anchors,
            links_wiki: true,
            ..Requirements::default()
        }
    }

    fn check(&self, ws: &Workspace, scope: &[PathBuf], out: &mut Vec<Diagnostic>) {
        for source in scope {
            let Some(links) = ws.links.get(source) else {
                continue;
            };
            for link in links {
                if !matches!(link.kind, LinkKind::Wiki) {
                    continue;
                }
                if link.target.is_empty() {
                    if let Some(anchor) = link.anchor.as_deref() {
                        if self.check_anchors
                            && !heading_text_matches(ws.headings.get(source), anchor)
                        {
                            out.push(missing_anchor_diagnostic(source, link.line, anchor));
                        }
                    }
                    continue;
                }

                match resolve(&link.target, ws) {
                    Resolution::Resolved(target_path) => {
                        if self.check_anchors {
                            check_anchor_against(source, link, &target_path, ws, out);
                        }
                    }
                    Resolution::Ambiguous(matches) => match self.on_ambiguous {
                        AmbiguityPolicy::FirstMatch => {
                            let pick = matches.iter().min().expect("non-empty");
                            if self.check_anchors {
                                check_anchor_against(source, link, pick, ws, out);
                            }
                        }
                        AmbiguityPolicy::Error | AmbiguityPolicy::Warn => {
                            out.push(ambiguous_diagnostic(
                                source,
                                link.line,
                                &link.target,
                                &matches,
                                matches!(self.on_ambiguous, AmbiguityPolicy::Error),
                            ));
                        }
                    },
                    Resolution::NotFound => {
                        out.push(missing_target_diagnostic(source, link.line, &link.target));
                    }
                }
            }
        }
    }
}

fn check_anchor_against(
    source: &Path,
    link: &LinkRef,
    target_path: &Path,
    ws: &Workspace,
    out: &mut Vec<Diagnostic>,
) {
    let Some(anchor) = link.anchor.as_deref() else {
        return;
    };
    if heading_text_matches(ws.headings.get(target_path), anchor) {
        return;
    }
    out.push(missing_anchor_diagnostic_for(
        source,
        link.line,
        &link.target,
        anchor,
    ));
}

/// Outcome of resolving one wikilink target against the workspace.
enum Resolution {
    /// Single unique file.
    Resolved(PathBuf),
    /// Multiple files share the shortest path depth among basename matches.
    Ambiguous(Vec<PathBuf>),
    /// No file matches.
    NotFound,
}

fn resolve(target: &str, ws: &Workspace) -> Resolution {
    // Drop a single trailing `.md` if the user wrote one. The remaining path is the
    // note's logical name; we must NOT call `file_stem` again because that would drop
    // a second extension and break notes whose basename contains dots
    // (e.g. `[[My.Note]]` should look up `my.note`, not `my`). The file's basename in
    // `Workspace::by_basename` is keyed via `file_stem` against the on-disk path,
    // which only strips the last `.md` — so the two key derivations stay symmetric.
    let cleaned = target.strip_suffix(".md").unwrap_or(target);
    let cleaned_path = Path::new(cleaned);
    let Some(name) = cleaned_path.file_name().and_then(|s| s.to_str()) else {
        return Resolution::NotFound;
    };
    let stem_lower = name.to_lowercase();

    let Some(candidates) = ws.by_basename.get(&stem_lower) else {
        return Resolution::NotFound;
    };
    if candidates.is_empty() {
        return Resolution::NotFound;
    }

    let parent_components: Vec<String> = cleaned_path
        .parent()
        .into_iter()
        .flat_map(Path::components)
        .filter_map(|c| c.as_os_str().to_str())
        .map(str::to_lowercase)
        .collect();

    let mut filtered: Vec<PathBuf> = candidates
        .iter()
        .filter(|file| matches_parent_components(file, &parent_components))
        .cloned()
        .collect();
    if filtered.is_empty() {
        return Resolution::NotFound;
    }

    let min_depth = filtered
        .iter()
        .map(|p| p.components().count())
        .min()
        .expect("non-empty");
    filtered.retain(|p| p.components().count() == min_depth);
    filtered.sort();

    if filtered.len() == 1 {
        Resolution::Resolved(filtered.into_iter().next().expect("len==1"))
    } else {
        Resolution::Ambiguous(filtered)
    }
}

/// True if `file`'s path components — excluding the basename — end with `parent` (case-
/// insensitive). An empty `parent` matches any file.
fn matches_parent_components(file: &Path, parent: &[String]) -> bool {
    if parent.is_empty() {
        return true;
    }
    let mut file_parents: Vec<String> = file
        .parent()
        .into_iter()
        .flat_map(Path::components)
        .filter_map(|c| c.as_os_str().to_str())
        .map(str::to_lowercase)
        .collect();
    if file_parents.len() < parent.len() {
        return false;
    }
    let tail = file_parents.split_off(file_parents.len() - parent.len());
    tail == parent
}

/// Factory that builds [`Rule`] from YAML params.
pub struct Factory;

impl WorkspaceRuleFactory for Factory {
    fn id(&self) -> &'static str {
        ID
    }

    fn build(&self, params: &serde_json::Value) -> Result<Box<dyn WorkspaceRule>, Error> {
        let raw: Params = if params.is_null() {
            Params::default()
        } else {
            serde_json::from_value(params.clone())
                .map_err(|e| Error::Schema(format!("invalid params for {ID}: {e}")))?
        };
        let on_ambiguous = match raw.on_ambiguous.as_deref() {
            None | Some("error") => AmbiguityPolicy::Error,
            Some("warn") => AmbiguityPolicy::Warn,
            Some("first-match") => AmbiguityPolicy::FirstMatch,
            Some(other) => {
                return Err(Error::Schema(format!(
                    "invalid on_ambiguous '{other}' for {ID} (expected error | warn | first-match)"
                )));
            }
        };
        Ok(Box::new(Rule {
            on_ambiguous,
            check_anchors: raw.check_anchors.unwrap_or(true),
        }))
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Params {
    #[serde(default)]
    on_ambiguous: Option<String>,
    #[serde(default)]
    check_anchors: Option<bool>,
}

fn heading_text_matches(headings: Option<&Vec<HeadingFact>>, anchor: &str) -> bool {
    headings.is_some_and(|hs| hs.iter().any(|h| h.text == anchor))
}

fn missing_target_diagnostic(source: &Path, line: usize, target: &str) -> Diagnostic {
    Diagnostic {
        file: source.to_path_buf(),
        line: Some(line),
        rule: ID,
        severity: Severity::Error,
        message: format!("wikilink target '{target}' not found in workspace"),
        fixit: None,
    }
}

fn missing_anchor_diagnostic(source: &Path, line: usize, anchor: &str) -> Diagnostic {
    Diagnostic {
        file: source.to_path_buf(),
        line: Some(line),
        rule: ID,
        severity: Severity::Error,
        message: format!("anchor '#{anchor}' has no matching heading"),
        fixit: None,
    }
}

fn missing_anchor_diagnostic_for(
    source: &Path,
    line: usize,
    target: &str,
    anchor: &str,
) -> Diagnostic {
    Diagnostic {
        file: source.to_path_buf(),
        line: Some(line),
        rule: ID,
        severity: Severity::Error,
        message: format!("wikilink target '{target}' has no heading matching anchor '#{anchor}'"),
        fixit: None,
    }
}

fn ambiguous_diagnostic(
    source: &Path,
    line: usize,
    target: &str,
    matches: &[PathBuf],
    error: bool,
) -> Diagnostic {
    let listed = matches
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    Diagnostic {
        file: source.to_path_buf(),
        line: Some(line),
        rule: ID,
        severity: if error {
            Severity::Error
        } else {
            Severity::Warning
        },
        message: format!(
            "wikilink target '{target}' is ambiguous; equally-shortest matches: {listed}"
        ),
        fixit: None,
    }
}

#[cfg(test)]
mod tests {
    use super::{matches_parent_components, resolve, Resolution};
    use mdtype_core::Workspace;
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    #[test]
    fn parent_match_empty_target_matches_anything() {
        assert!(matches_parent_components(Path::new("a/b/c.md"), &[]));
    }

    #[test]
    fn parent_match_suffix_segments() {
        let parent = vec!["sub".to_string(), "dir".to_string()];
        assert!(matches_parent_components(
            Path::new("vault/sub/dir/note.md"),
            &parent
        ));
        assert!(!matches_parent_components(
            Path::new("vault/other/dir/note.md"),
            &parent
        ));
    }

    #[test]
    fn parent_match_is_case_insensitive() {
        let parent = vec!["sub".to_string(), "dir".to_string()];
        assert!(matches_parent_components(
            Path::new("Vault/Sub/Dir/note.md"),
            &parent
        ));
    }

    fn workspace_with(files: &[&str]) -> Workspace {
        let mut ws = Workspace::default();
        for f in files {
            let path = PathBuf::from(f);
            ws.files.push(path.clone());
            if let Some(stem) = path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(str::to_lowercase)
            {
                ws.by_basename.entry(stem).or_default().push(path);
            }
        }
        ws.headings = HashMap::new();
        ws
    }

    #[test]
    fn dotted_basename_resolves() {
        // Regression: an Obsidian note named `My.Note.md` linked as `[[My.Note]]` must
        // resolve. The previous resolver called Path::file_stem on the already-cleaned
        // target, dropping `.Note` and looking up `my` instead of `my.note`.
        let ws = workspace_with(&["notes/My.Note.md"]);
        match resolve("My.Note", &ws) {
            Resolution::Resolved(p) => assert_eq!(p, PathBuf::from("notes/My.Note.md")),
            other => panic!("expected Resolved, got {:?}", debug_resolution(&other)),
        }
        // Same target but written with the trailing `.md` still resolves.
        match resolve("My.Note.md", &ws) {
            Resolution::Resolved(p) => assert_eq!(p, PathBuf::from("notes/My.Note.md")),
            other => panic!("expected Resolved, got {:?}", debug_resolution(&other)),
        }
    }

    fn debug_resolution(r: &Resolution) -> &'static str {
        match r {
            Resolution::Resolved(_) => "Resolved",
            Resolution::Ambiguous(_) => "Ambiguous",
            Resolution::NotFound => "NotFound",
        }
    }
}
