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
    pub input: Arc<dyn PhysicalPlan>,
    pub partition_expr: Vec<Arc<dyn Expression>>,
    pub job_uuid: String,
    pub stage_id: i32,
    pub partition_count: i32,
}

impl ShuffleWriterExec {
    pub fn new(
        input: Arc<dyn PhysicalPlan>,
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

    fn children(&self) -> Vec<&Arc<dyn PhysicalPlan>> {
        vec![&self.input]
    }

    /// Rebuild this shuffle writer with a new input child. See the trait-level
    /// `PhysicalPlan::with_new_children` doc for the general rewrite pattern.
    ///
    /// Arity 1: a shuffle writer wraps exactly one input — the operator whose
    /// output will be hash-partitioned and written to local shuffle files.
    /// `into_iter().next().unwrap()` consumes the length-1 children vec and
    /// takes ownership of that single Arc.
    ///
    /// The shuffle-identifying fields (`partition_expr`, `job_uuid`,
    /// `stage_id`, `partition_count`) are reused — they describe where this
    /// stage's output goes, which is independent of which concrete input
    /// produces the rows.
    fn with_new_children(
        self: Arc<Self>,
        children: Vec<Arc<dyn PhysicalPlan>>,
    ) -> Arc<dyn PhysicalPlan> {
        assert_eq!(
            children.len(),
            1,
            "ShuffleWriterExec expects exactly 1 child"
        );
        Arc::new(ShuffleWriterExec::new(
            children.into_iter().next().unwrap(),
            self.partition_expr.clone(),
            self.job_uuid.clone(),
            self.stage_id,
            self.partition_count,
        ))
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
