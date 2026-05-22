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
//! - `LogicalPlan` and `LogicalExpr` are Rust `enum`s. In Kotlin each is a
//!   plain `interface` implemented by a fixed set of classes (one class per
//!   operator / expression, no `sealed` keyword); the closed implementor set
//!   collapses cleanly into one enum with a variant per class.
//! - Each operator keeps its own file (`scan.rs`, `projection.rs`, …) holding
//!   a struct plus its `schema` / `children` / `Display` logic; `logical_plan.rs`
//!   holds the `LogicalPlan` enum that dispatches to them.
//! - Aggregate functions are a separate `AggregateExpr` enum (`Sum` / `Min` /
//!   `Max` / `Avg` / `Count` / `CountDistinct`, in `expressions.rs`), so the
//!   `Aggregate` plan keeps a typed `Vec<AggregateExpr>` (Kotlin
//!   `List<AggregateExpr>`). A single bridge variant
//!   `LogicalExpr::AggregateExpr(Box<AggregateExpr>)` (with
//!   `From<AggregateExpr> for LogicalExpr`) injects an aggregate into the
//!   expression enum so it can nest inside any expression — e.g. a `HAVING`
//!   predicate. This mirrors Kotlin's `AggregateExpr : LogicalExpr` and
//!   DataFusion's `Expr::AggregateFunction`.
//! - `DataFrame` is a fluent, `self`-consuming builder wrapping a `LogicalPlan`.
//!
//! ## Status
//! Module 3 of 15 — ported. All 10 Kotlin source files have Rust equivalents.

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

// ==============================================================
// Re-exports for convenient downstream `use logical_plan::*;` ergonomics.
// ==============================================================
pub use aggregate::Aggregate;
pub use data_frame::DataFrame;
pub use expressions::{
    avg, cast, col, count, count_distinct, lit_date, lit_double, lit_float, lit_long, lit_string,
    max, min, sum, AggregateExpr,
};
pub use join::{Join, JoinType};
pub use limit::Limit;
pub use logical_expr::LogicalExpr;
pub use logical_plan::{format, LogicalPlan};
pub use projection::Projection;
pub use scan::Scan;
pub use selection::Selection;
