//! # query-planner
//!
//! Translation from `LogicalPlan` to `PhysicalPlan`.
//!
//! ## Kotlin source
//! Faithful port of `kquery/query-planner/src/main/kotlin/`:
//! `QueryPlanner.kt`.
//!
//! Smallest module — a single `QueryPlanner` type with two `match` functions:
//! `create_physical_plan(&LogicalPlan) -> Box<dyn PhysicalPlan>` and
//! `create_physical_expr(&LogicalExpr, &LogicalPlan) -> Arc<dyn Expression>`.
//!
//! ## Status
//! Module 7 of 15 — ported. `QueryPlanner.kt` has a Rust equivalent and
//! `QueryPlannerTest` is ported (as structural assertions — see the test note in
//! `query_planner.rs`). Both `match`es are exhaustive over the closed
//! `LogicalPlan`/`LogicalExpr` enums; variants with no physical counterpart panic
//! with the same message Kotlin's `else -> throw` carried.

// ==============================================================
// Per-file modules — one for each upstream Kotlin source file.
// ==============================================================
pub mod query_planner;

pub use query_planner::QueryPlanner;
