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
//! Module 5 of 15 — ported. Both Kotlin source files have Rust equivalents, and
//! the `OptimizerTest` suite is ported as a `#[cfg(test)]` module.

// ==============================================================
// Per-file modules — one for each upstream Kotlin source file.
// ==============================================================
pub mod optimizer;
pub mod projection_push_down_rule;
