//! # query-planner
//!
//! Translation from `LogicalPlan` to `PhysicalPlan`.
//!
//! ## Kotlin source
//! Faithful port of `kquery/query-planner/src/main/kotlin/`:
//! `QueryPlanner.kt`.
//!
//! Smallest module — single `QueryPlanner` type with one `create_physical_plan(&LogicalPlan)`
//! entry point. The Kotlin original is ~150 LOC.
//!
//! ## Status
//! TODO — module 7 of 15.

// ==============================================================
// Per-file modules — one for each upstream Kotlin source file.
// ==============================================================
pub mod query_planner;
