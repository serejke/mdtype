//! Core types and traits for `mdtype`.
//!
//! This crate defines the data model, trait boundaries, parser, and default validator
//! used by the CLI and all sibling crates. It has zero dependencies on sibling crates,
//! so downstream users may depend on `mdtype-core` alone and supply their own
//! schema source, rules, and reporter.

#![forbid(unsafe_code)]

pub mod diagnostic;
pub mod error;
pub mod extract;
pub mod parser;
pub mod rule;
pub mod runner;
pub mod schema;
pub mod source;
pub mod validator;
pub mod workspace;

pub use diagnostic::{Diagnostic, Fixit, Severity, Summary};
pub use error::Error;
pub use parser::{
    parse_file, parse_file_with_options, read_frontmatter, split_frontmatter, ParsedDocument,
};
pub use rule::{BodyRule, BodyRuleFactory, WorkspaceRule, WorkspaceRuleFactory};
pub use runner::{run_workspace, RUNNER_PARSE_RULE_ID};
pub use schema::{ReferenceSpec, Schema};
pub use source::{SchemaEntry, SchemaSource};
pub use validator::{CoreValidator, Reporter, Validator, FRONTMATTER_RULE_ID};
pub use workspace::{HeadingFact, LinkKind, LinkRef, Requirements, Workspace};

// Re-export comrak so downstream rule crates can inspect the AST without a direct dep.
pub use comrak;
pub use comrak::nodes;
pub use comrak::Arena;
