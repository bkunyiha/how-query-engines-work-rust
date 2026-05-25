//! Port of `kquery/physical-plan/src/main/kotlin/Action.kt`.
//!
//! Wire-protocol action types for distributed execution: a request is either a
//! whole query (`QueryAction`) or a request for a previously-computed shuffle
//! partition (`ShuffleIdAction`). These are scaffolding for the `distributed` /
//! `flight-server` modules; `ShuffleIdAction` in particular is defined-but-unused
//! in kquery, and is ported here as the dead-but-defined type it is upstream.
//!
//! ## Translation note
//! Kotlin `interface Action` with two `data class` implementors becomes a marker
//! `trait Action` plus two structs. `QueryAction` can derive only `Clone`, because
//! `logical_plan::LogicalPlan` derives only `Clone` (no `Debug`/`PartialEq`).

use datatypes::ShuffleId;
use logical_plan::LogicalPlan;

/// Marker trait for a distributed-execution action. Kotlin `interface Action`.
pub trait Action {}

/// Execute a whole logical plan. Kotlin `data class QueryAction(val plan: LogicalPlan)`.
#[derive(Clone)]
pub struct QueryAction {
    pub plan: LogicalPlan,
}

impl QueryAction {
    pub fn new(plan: LogicalPlan) -> Self {
        Self { plan }
    }
}

impl Action for QueryAction {}

/// Fetch a previously-produced shuffle partition. Kotlin
/// `data class ShuffleIdAction(val shuffleId: ShuffleId)`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShuffleIdAction {
    pub shuffle_id: ShuffleId,
}

impl ShuffleIdAction {
    pub fn new(shuffle_id: ShuffleId) -> Self {
        Self { shuffle_id }
    }
}

impl Action for ShuffleIdAction {}
