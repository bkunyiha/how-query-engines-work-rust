//! # execution
//!
//! The runtime — `ExecutionContext` and `ParallelContext`. End of the
//! single-node Phase-1 build order (modules 1–8 produce a working in-process
//! query engine that can run TPC-H Q1; see Plan §3.5).
//!
//! ## Kotlin source
//! Faithful port of `kquery/execution/src/main/kotlin/`:
//! `ExecutionContext.kt`, `ParallelContext.kt`.
//!
//! ## Status
//! TODO — module 8 of 15. **Milestone:** working in-process engine at end.

// ==============================================================
// Per-file modules — one for each upstream Kotlin source file.
// ==============================================================
pub mod execution_context;
pub mod parallel_context;
