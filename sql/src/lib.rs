//! # sql
//!
//! Hand-ported Pratt parser and SQL → LogicalPlan compiler.
//!
//! ## Kotlin source
//! Faithful port of `kquery/sql/src/main/kotlin/`: `Tokens.kt`,
//! `SqlTokenizer.kt`, `TokenStream.kt`, `PrattParser.kt`, `SqlParser.kt`,
//! `Expressions.kt` (the SQL AST), `SqlPlanner.kt`.
//!
//! ## ⚠ Design directive
//! **DO NOT replace the Pratt parser with `sqlparser-rs`.** The
//! hand-port is the pedagogical core of this module. Swapping in sqlparser-rs
//! is a Phase 2 (`fdapquery`) decision, not a Phase 1 one.
//!
//! ## Status
//! Module 4 of 15 — ported. All 7 Kotlin source files have Rust equivalents,
//! with the tokenizer / parser / planner test suites ported as `#[cfg(test)]`
//! modules.

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

// ==============================================================
// Re-exports for ergonomic `use sql::*;`. The Pratt parser trait must
// be re-exported alongside `SqlParser` because parse() lives on the
// trait — anyone calling `parser.parse(0)` needs both in scope.
// ==============================================================
pub use expressions::SqlExpr;
pub use pratt_parser::PrattParser;
pub use sql_parser::SqlParser;
pub use sql_planner::SqlPlanner;
pub use sql_tokenizer::SqlTokenizer;
