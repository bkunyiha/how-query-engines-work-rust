//! Port of `kquery/physical-plan/src/main/kotlin/ShuffleWriterExec.kt`.
//!
//! Executes its input and writes the output to local shuffle files, partitioned by
//! the hash of a set of partition expressions. Used at shuffle boundaries in
//! distributed execution.
//!
//! ## Status — stubbed until the distributed module (§4.6)
//! As in Kotlin, the standard `execute()` is unsupported: writing shuffle output
//! needs the executor context (executor id/host/port + a `ShuffleManager`). Kotlin
//! throws `UnsupportedOperationException` and exposes
//! `executeAndWriteShuffle(...)`. The Rust port keeps the struct and the
//! `PhysicalPlan` surface; `execute()` is `unimplemented!()`, and the actual
//! hash-partition-and-write is completed with the `distributed` module (it depends
//! on `ShuffleManager`'s Arrow-IPC writing).

use crate::expressions::Expression;
use crate::physical_plan::PhysicalPlan;
use datatypes::{RecordBatch, Schema};
use std::sync::Arc;

/// Partitions input by hash and writes shuffle output. Kotlin `ShuffleWriterExec`.
pub struct ShuffleWriterExec {
    pub input: Box<dyn PhysicalPlan>,
    pub partition_expr: Vec<Arc<dyn Expression>>,
    pub job_uuid: String,
    pub stage_id: i32,
    pub partition_count: i32,
}

impl ShuffleWriterExec {
    pub fn new(
        input: Box<dyn PhysicalPlan>,
        partition_expr: Vec<Arc<dyn Expression>>,
        job_uuid: impl Into<String>,
        stage_id: i32,
        partition_count: i32,
    ) -> Self {
        Self {
            input,
            partition_expr,
            job_uuid: job_uuid.into(),
            stage_id,
            partition_count,
        }
    }
}

impl PhysicalPlan for ShuffleWriterExec {
    fn schema(&self) -> Schema {
        self.input.schema()
    }

    fn children(&self) -> Vec<&dyn PhysicalPlan> {
        vec![self.input.as_ref()]
    }

    fn execute(&self) -> Box<dyn Iterator<Item = RecordBatch>> {
        // Kotlin throws UnsupportedOperationException; the hash-partition-and-write
        // (executeAndWriteShuffle) lands with the distributed module.
        unimplemented!(
            "ShuffleWriterExec::execute() must be driven by the distributed executor \
             (hash-partition + write via ShuffleManager); completed in module 13/14"
        )
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl std::fmt::Display for ShuffleWriterExec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let exprs: Vec<String> = self.partition_expr.iter().map(|e| e.to_string()).collect();
        write!(
            f,
            "ShuffleWriterExec: jobUuid={}, stageId={}, partitionCount={}, partitionExpr=[{}]",
            self.job_uuid,
            self.stage_id,
            self.partition_count,
            exprs.join(", ")
        )
    }
}
