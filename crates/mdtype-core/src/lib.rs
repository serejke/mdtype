//! Core types and traits for `mdtype`.
//!
//! This crate defines the data model, trait boundaries, parser, and default validator
//! used by the CLI and all sibling crates. It has zero dependencies on sibling crates,
//! so downstream users may depend on `mdtype-core` alone and supply their own
//! schema source, rules, and reporter.

#![forbid(unsafe_code)]

pub mod diagnostic;
pub mod error;
pub mod parser;
pub mod rule;
pub mod schema;
pub mod source;
pub mod validator;

pub use diagnostic::{Diagnostic, Fixit, Severity, Summary};
pub use error::Error;
pub use parser::{parse_file, ParsedDocument};
pub use rule::{BodyRule, BodyRuleFactory};
pub use schema::Schema;
pub use source::{SchemaEntry, SchemaSource};
pub use validator::{CoreValidator, Reporter, Validator};
