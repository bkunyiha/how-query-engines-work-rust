//! # execution
//!
//! The runtime — `ExecutionContext` and `ParallelContext`. End of the
//! single-node Phase-1 build order (modules 1–8 produce a working in-process
//! query engine that can run TPC-H Q1).
//!
//! ## Kotlin source
//! Faithful port of `kquery/execution/src/main/kotlin/`:
//! `ExecutionContext.kt`, `ParallelContext.kt`.
//!
//! ## Status
//! Module 8 of 15 — ported. **Milestone reached:** a working in-process query
//! engine. `ExecutionContext::sql(...)` / `execute(...)` runs a query end-to-end
//! (parse → optimize → plan → execute → `Vec<RecordBatch>`), and `ParallelContext`
//! runs aggregate queries across rayon workers (the faithful substitute for
//! Kotlin's coroutines — ARCHITECTURE.md §3.9 / §4.8). Both Kotlin source files
//! have Rust equivalents; `ExecutionSqlTest`, the deterministic non-`Fuzzer` cases
//! of `ExecutionTest`, and `ParallelContextTest` are ported as `#[cfg(test)]`
//! modules. (The `Fuzzer`-backed `ExecutionTest` cases land with module 9.)

// ==============================================================
// Per-file modules — one for each upstream Kotlin source file.
// ==============================================================
pub mod execution_context;
pub mod parallel_context;

// ==============================================================
// Re-exports for `use execution::*;` ergonomics.
// ==============================================================
pub use execution_context::ExecutionContext;
pub use parallel_context::ParallelContext;
