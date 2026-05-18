//! # logical-plan
//!
//! Logical plan tree and DataFrame API.
//!
//! ## Kotlin source
//! Faithful port of `kquery/logical-plan/src/main/kotlin/`:
//! `LogicalPlan.kt`, `LogicalExpr.kt`, `DataFrame.kt`, `Scan.kt`,
//! `Projection.kt`, `Selection.kt`, `Aggregate.kt`, `Join.kt`, `Limit.kt`,
//! `Expressions.kt`.
//!
//! ## Design
//! - `LogicalPlan` and `LogicalExpr` are Rust `enum`s (sealed in Kotlin).
//! - Pattern-match exhaustively in the optimiser and physical planner.
//! - `DataFrame` is a thin builder wrapping `LogicalPlan`.
//!
//! ## Status
//! TODO — module 3 of 15.

// ==============================================================
// Per-file modules — one for each upstream Kotlin source file.
// ==============================================================
pub mod aggregate;
pub mod data_frame;
pub mod expressions;
pub mod join;
pub mod limit;
pub mod logical_expr;
pub mod logical_plan;
pub mod projection;
pub mod scan;
pub mod selection;
