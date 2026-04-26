//! Schema-derived type checks.
//!
//! Unlike rules under `body:` and `workspace:` (which the user explicitly enables in
//! YAML), the checks in this module are **declaration-driven**: they fire because a
//! schema declares them inline, not because the user lists a rule. They are installed
//! into a schema's workspace pipeline by
//! [`install_type_checks`](crate::install_type_checks). See `docs/types.md`.

pub mod entity_ref;
