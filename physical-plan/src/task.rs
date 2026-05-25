//! Port of `kquery/physical-plan/src/main/kotlin/Task.kt`.
//!
//! A unit of distributed work: a physical plan to run for one partition of one
//! stage of a job. Scaffolding for the `distributed` module. The Kotlin source
//! keeps the equivalent protobuf message as a comment; we preserve it.
//!
//! ## Translation note
//! Kotlin's `data class Task` can't be a Rust `data`-style struct with derives:
//! `plan: PhysicalPlan` becomes `Box<dyn PhysicalPlan>`, which is neither `Clone`,
//! `Debug`, nor `PartialEq`, so no derives are applied.

use crate::physical_plan::PhysicalPlan;

/*
 message Task {
   string job_uuid = 1;
   uint32 stage_id = 2;
   uint32 task_id = 3;
   uint32 partition_id = 4;
   PhysicalPlanNode plan = 5;
   // The task could need to read shuffle output from another task
   repeated ShuffleLocation shuffle_loc = 6;
 }
*/

/// A distributed task. Kotlin `data class Task(jobUuid, stageId, taskId, partitionId, plan)`.
pub struct Task {
    pub job_uuid: String,
    pub stage_id: i32,
    pub task_id: i32,
    pub partition_id: i32,
    pub plan: Box<dyn PhysicalPlan>,
}

impl Task {
    pub fn new(
        job_uuid: impl Into<String>,
        stage_id: i32,
        task_id: i32,
        partition_id: i32,
        plan: Box<dyn PhysicalPlan>,
    ) -> Self {
        Self {
            job_uuid: job_uuid.into(),
            stage_id,
            task_id,
            partition_id,
            plan,
        }
    }
}
