//! Helpers for `mdtype-tests` integration tests.
//!
//! The crate intentionally has no production code — its unit of work lives in `tests/`.
//! Helpers added here over time should stay test-only utilities (binary discovery, fixture
//! walkers, snapshot path conventions).

#![forbid(unsafe_code)]
