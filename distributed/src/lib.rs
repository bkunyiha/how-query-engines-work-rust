//! # distributed
//!
//! Distributed execution layer — scheduler, query-stage decomposition,
//! distributed planner. The "minimal Ballista" example in Plan §10 chapter 12.
//!
//! ## Kotlin source
//! Faithful port of `kquery/distributed/src/main/kotlin/io/`:
//! `DistributedContext.kt`, `DistributedConfig.kt`, `DistributedPlanner.kt`,
//! `QueryStage.kt`, `Scheduler.kt`.
//!
//! ## Status
//! TODO — module 15 of 15. Last module in the build order.

// ==============================================================
// Per-file modules — one for each upstream Kotlin source file.
// ==============================================================
pub mod distributed_config;
pub mod distributed_context;
pub mod distributed_planner;
pub mod query_stage;
pub mod scheduler;
