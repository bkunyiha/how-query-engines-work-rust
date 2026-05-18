//! # optimizer
//!
//! Logical optimisation rules applied to `LogicalPlan` trees.
//!
//! ## Kotlin source
//! Faithful port of `kquery/optimizer/src/main/kotlin/`:
//! `Optimizer.kt`, `ProjectionPushDownRule.kt`.
//!
//! ## Design
//! - `OptimizerRule` is a trait with `fn optimize(&self, plan: &LogicalPlan) -> LogicalPlan`.
//! - Each rule is a stateless struct.
//! - Rules apply in a fixed order in Phase 1 (the upstream kquery does the same;
//!   cost-based ordering is a Phase 2 / DataFusion-territory concern).
//!
//! ## Status
//! TODO — module 5 of 15.

// ==============================================================
// Per-file modules — one for each upstream Kotlin source file.
// ==============================================================
pub mod optimizer;
pub mod projection_push_down_rule;
