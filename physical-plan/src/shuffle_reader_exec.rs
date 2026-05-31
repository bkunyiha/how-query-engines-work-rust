//! Port of `kquery/physical-plan/src/main/kotlin/ShuffleReaderExec.kt`.
//!
//! Reads shuffle output from one or more [`ShuffleLocation`]s at the start of a
//! stage that consumes a previous stage's output (local files for data on this
//! executor, Arrow Flight for remote executors).
//!
//! ## Status — stubbed until the distributed module (§4.6)
//! Like its Kotlin original, the standard `execute()` is unsupported here: a shuffle
//! read needs the executor context (which executor am I? which `ShuffleManager`?
//! which Flight client?). Kotlin throws `UnsupportedOperationException` and exposes
//! a separate `executeWithContext(...)`. The Rust port keeps the struct and the
//! `PhysicalPlan` surface; `execute()` is `unimplemented!()`, and the
//! context-driven read is completed alongside the `distributed`/`flight-server`
//! modules (it depends on `ShuffleManager` IO and an Arrow Flight client).

use crate::physical_plan::PhysicalPlan;
use crate::shuffle_location::ShuffleLocation;
use datatypes::{RecordBatch, Schema};

/// Reads shuffle data from a set of locations. Kotlin `ShuffleReaderExec`.
pub struct ShuffleReaderExec {
    pub shuffle_schema: Schema,
    pub shuffle_locations: Vec<ShuffleLocation>,
}

impl ShuffleReaderExec {
    pub fn new(shuffle_schema: Schema, shuffle_locations: Vec<ShuffleLocation>) -> Self {
        Self {
            shuffle_schema,
            shuffle_locations,
        }
    }
}

impl PhysicalPlan for ShuffleReaderExec {
    fn schema(&self) -> Schema {
        self.shuffle_schema.clone()
    }

    fn children(&self) -> Vec<&dyn PhysicalPlan> {
        // A shuffle read is a leaf — its input is the previous stage's output.
        vec![]
    }

    fn execute(&self) -> Box<dyn Iterator<Item = RecordBatch>> {
        // Kotlin throws UnsupportedOperationException; the context-driven read
        // (local via ShuffleManager, remote via Arrow Flight) lands with the
        // distributed module.
        unimplemented!(
            "ShuffleReaderExec::execute() must be driven by the distributed executor \
             (local reads via ShuffleManager, remote via Arrow Flight); completed in module 13/14"
        )
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl std::fmt::Display for ShuffleReaderExec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ShuffleReaderExec: schema={:?}, locations={}",
            self.shuffle_schema,
            self.shuffle_locations.len()
        )
    }
}
