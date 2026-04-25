//! Obsidian-flavored workspace rules for `mdtype`.
//!
//! Resolves wikilinks (`[[Target]]`, `[[Target|Alias]]`, `[[Target#Heading]]`) using
//! Obsidian's policy: exact path → basename → shortest-path tiebreak.

#![forbid(unsafe_code)]

pub mod links_obsidian_vault;

use mdtype_core::WorkspaceRuleFactory;

/// Return the workspace-rule factories shipped by this crate.
#[must_use]
pub fn register_obsidian() -> Vec<Box<dyn WorkspaceRuleFactory>> {
    vec![Box::new(links_obsidian_vault::Factory)]
}
