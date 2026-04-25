//! Rule traits and factory interfaces.
//!
//! Two flavors:
//!
//! - [`BodyRule`] examines a single [`ParsedDocument`] and appends diagnostics. This is
//!   the per-file rule trait that has existed since v0.1; existing rules implement it
//!   unchanged.
//! - [`WorkspaceRule`] examines the whole [`Workspace`] and appends diagnostics for files
//!   in its `scope`. Workspace rules answer cross-file questions like "does this link
//!   resolve?" or "is this basename ambiguous?".
//!
//! Both kinds register via factories so YAML loaders can construct them from rule ids.

use std::path::PathBuf;

use crate::diagnostic::Diagnostic;
use crate::error::Error;
use crate::parser::ParsedDocument;
use crate::workspace::{Requirements, Workspace};

/// A body-structure check applied to a parsed document.
pub trait BodyRule: Send + Sync {
    /// Stable identifier, e.g. `"body.required_sections"`. Becomes the `rule` field on diagnostics.
    fn id(&self) -> &'static str;

    /// Run the rule against `doc` and append any findings to `out`.
    fn check(&self, doc: &ParsedDocument, out: &mut Vec<Diagnostic>);
}

/// Constructs a `BodyRule` from the YAML parameters attached to its entry in a schema file.
///
/// Schema loaders look up the factory for a rule id, pass the rule's YAML node, and receive
/// a fully configured rule instance.
pub trait BodyRuleFactory: Send + Sync {
    /// The rule id this factory produces. Matches `BodyRule::id`.
    fn id(&self) -> &'static str;

    /// Build a rule from its YAML parameters, or return `Err(Error::Schema(_))`.
    ///
    /// # Errors
    ///
    /// Returns `Error::Schema` if the params are missing required fields or have the wrong shape.
    fn build(&self, params: &serde_json::Value) -> Result<Box<dyn BodyRule>, Error>;
}

/// A cross-file check applied to the assembled [`Workspace`].
///
/// Workspace rules are pure judges: they read facts the runner has already extracted and
/// emit diagnostics. They never mutate `Workspace` themselves; required fact kinds are
/// declared via [`requires`](Self::requires) and gathered by core extractors.
pub trait WorkspaceRule: Send + Sync {
    /// Stable identifier, e.g. `"links.relative_path"`. Becomes the `rule` field on
    /// every diagnostic this rule emits.
    fn id(&self) -> &'static str;

    /// What this rule needs the runner to gather. Default: nothing — relies entirely on
    /// the always-on facts (`Workspace::files` and `Workspace::by_basename`).
    fn requires(&self) -> Requirements {
        Requirements::default()
    }

    /// Read facts from `ws`, emit diagnostics for files in `scope`.
    ///
    /// `scope` is the slice of files for which this rule instance is enabled — i.e.,
    /// files whose attached schema lists this exact rule entry. The rule must only emit
    /// diagnostics for files inside `scope`; it may freely read any fact in `ws` (a link
    /// in a scope file may resolve to an out-of-scope file, and the resolution is
    /// expected to succeed because facts are gathered globally).
    fn check(&self, ws: &Workspace, scope: &[PathBuf], out: &mut Vec<Diagnostic>);
}

/// Constructs a [`WorkspaceRule`] from the YAML parameters attached to its entry in a
/// schema file. Mirrors [`BodyRuleFactory`].
pub trait WorkspaceRuleFactory: Send + Sync {
    /// The rule id this factory produces. Matches [`WorkspaceRule::id`].
    fn id(&self) -> &'static str;

    /// Build a rule from its YAML parameters, or return `Err(Error::Schema(_))`.
    ///
    /// # Errors
    ///
    /// Returns `Error::Schema` if the params are missing required fields or have the
    /// wrong shape.
    fn build(&self, params: &serde_json::Value) -> Result<Box<dyn WorkspaceRule>, Error>;
}
