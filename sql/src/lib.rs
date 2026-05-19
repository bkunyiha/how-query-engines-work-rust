//! # sql
//!
//! Hand-ported Pratt parser and SQL → LogicalPlan compiler.
//!
//! ## Kotlin source
//! Faithful port of `kquery/sql/src/main/kotlin/`:
//! `SqlTokenizer.kt`, `PrattParser.kt`, `SqlParser.kt`, `SqlPlanner.kt`,
//! `Expressions.kt`.
//!
//! ## ⚠ Design directive
//! **DO NOT replace the Pratt parser with `sqlparser-rs`.** The
//! hand-port is the pedagogical core of this module. Swapping in sqlparser-rs
//! is a Phase 2 (`fdapquery`) decision, not a Phase 1 one.
//!
//! ## Status
//! TODO — module 4 of 15.

// ==============================================================
// Per-file modules — one for each upstream Kotlin source file.
// ==============================================================
pub mod expressions;
pub mod pratt_parser;
pub mod sql_parser;
pub mod sql_planner;
pub mod sql_tokenizer;
pub mod token_stream;
pub mod tokens;
