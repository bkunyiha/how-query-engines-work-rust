//! # distributed
//!
//! Distributed query execution layer — scheduler, query-stage decomposition,
//! distributed planner, distributed context facade. The "minimal Ballista"
//! example from book chapter 12 of *How Query Engines Work*.
//!
//! ## Kotlin source
//! Faithful port of `kquery/distributed/src/main/kotlin/io/andygrove/kquery/distributed/`:
//! - `DistributedConfig.kt` — `ExecutorConfig`, `DistributedConfig` data classes
//! - `QueryStage.kt` — `QueryStage` (and unused `StageResult`, which we skip)
//! - `DistributedPlanner.kt` — splits a single-node physical plan into stages at
//!   shuffle boundaries (currently only the two-stage aggregate pattern)
//! - `Scheduler.kt` — `Scheduler` + the `ExecutorClient` abstraction boundary to
//!   the Flight world
//! - `DistributedContext.kt` — facade matching `ExecutionContext`'s public API
//!   (`register_csv` / `register` / `sql` / `execute`)
//!
//! ## Architectural notes
//! - **Synchronous, sequential** — like the upstream Kotlin. No async, no Tokio,
//!   no rayon. Each stage runs in dependency order; each task within a stage is
//!   dispatched one-at-a-time, round-robin across executors. This is deliberate:
//!   the module is a teaching artifact, not a production scheduler. Async lives
//!   one layer up at the Flight boundary (`flight-server` / `client`).
//! - **`ExecutorClient` is the seam to Flight.** The trait has three methods
//!   (`execute_task` / `execute_final_task` / `fetch_shuffle`). This crate ships
//!   the trait but not a real implementation; `MockExecutorClient` in tests proves
//!   the scheduler is exercisable without Flight. The real implementation lands
//!   with module 13 (`flight-server` / `client`).
//! - **No `protobuf` dep.** Verified: kquery's distributed module imports zero
//!   protobuf types. Wire serialisation only happens at the Flight boundary.
//!
//! ## Status
//! Module 15 of 15 — the last module in the workspace build order. Ported out
//! of build order: distributed (15) was completed first because the
//! `ExecutorClient` trait and `Scheduler` types it defines are needed to drive
//! the integration tests for flight-server (13) and client (14). Built against
//! the un-stubbed `ShuffleManager` in `physical-plan`; the `ShuffleReaderExec`
//! / `ShuffleWriterExec` `execute()` bodies were stubbed at module-14 boundary
//! and un-stubbed in module 13 when `ExecutorContext` landed.

pub mod distributed_config;
pub mod distributed_context;
pub mod distributed_planner;
pub mod query_stage;
pub mod scheduler;

// Re-exports for ergonomic `use distributed::*;` are added per-batch as the
// types come into existence.
pub use distributed_config::{DistributedConfig, ExecutorConfig};
pub use distributed_context::DistributedContext;
pub use distributed_planner::DistributedPlanner;
pub use query_stage::QueryStage;
pub use scheduler::{ExecutorClient, Scheduler};
