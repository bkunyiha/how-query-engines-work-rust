//! Port of `kquery/physical-plan/src/main/kotlin/ShuffleManager.kt`.
//!
//! Manages local shuffle storage for distributed execution, laid out as
//! `{base_dir}/{job_uuid}/{stage_id}/partition_{partition_id}.arrow`. The path and
//! cleanup helpers are pure filesystem operations and are ported here; the
//! Arrow-IPC `write_partition`/`read_partition` bodies are **deferred** —
//! shuffle is only exercised once the `distributed` / `flight-server` modules
//! (12–14) drive it, and those reads/writes go through Arrow IPC files (which a
//! later step wires up with the `arrow-ipc` crate). They are stubbed with
//! `unimplemented!()` until then (ARCHITECTURE.md §4.6).

use datatypes::RecordBatch;
use std::path::PathBuf;

/// Local shuffle-file storage manager. Kotlin `class ShuffleManager(baseDir)`.
pub struct ShuffleManager {
    pub base_dir: String,
}

impl Default for ShuffleManager {
    fn default() -> Self {
        // Default shuffle-spill directory. The Kotlin original used
        // "/tmp/kquery-shuffle"; the Rust port renames this to the project's name
        // (see TRANSLATION_NOTES.md "User-visible strings renamed kquery -> rquery").
        Self::new("/tmp/rquery-shuffle")
    }
}

impl ShuffleManager {
    pub fn new(base_dir: impl Into<String>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    /// Write a partition's batches to a local Arrow-IPC file.
    ///
    /// Deferred: completed with the distributed module (Arrow-IPC file writing).
    #[allow(unused_variables)]
    pub fn write_partition(
        &self,
        job_uuid: &str,
        stage_id: i32,
        partition_id: i32,
        batches: &[RecordBatch],
    ) {
        unimplemented!(
            "ShuffleManager::write_partition is completed with the distributed module (Arrow-IPC file IO)"
        )
    }

    /// Read a partition's batches from local Arrow-IPC storage.
    ///
    /// Deferred: completed with the distributed module (Arrow-IPC file reading).
    #[allow(unused_variables)]
    pub fn read_partition(
        &self,
        job_uuid: &str,
        stage_id: i32,
        partition_id: i32,
    ) -> Box<dyn Iterator<Item = RecordBatch>> {
        unimplemented!(
            "ShuffleManager::read_partition is completed with the distributed module (Arrow-IPC file IO)"
        )
    }

    /// Path of a shuffle partition file. Kotlin `getPartitionFile`.
    pub fn get_partition_file(&self, job_uuid: &str, stage_id: i32, partition_id: i32) -> PathBuf {
        self.partition_dir(job_uuid, stage_id)
            .join(format!("partition_{partition_id}.arrow"))
    }

    /// Directory for one job/stage. Kotlin private `getPartitionDir`.
    fn partition_dir(&self, job_uuid: &str, stage_id: i32) -> PathBuf {
        PathBuf::from(&self.base_dir)
            .join(job_uuid)
            .join(stage_id.to_string())
    }

    /// Remove all shuffle data for a job. Kotlin `cleanupJob`.
    pub fn cleanup_job(&self, job_uuid: &str) {
        let dir = PathBuf::from(&self.base_dir).join(job_uuid);
        if dir.exists() {
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    /// Remove all shuffle data. Kotlin `cleanupAll`.
    pub fn cleanup_all(&self) {
        let dir = PathBuf::from(&self.base_dir);
        if dir.exists() {
            let _ = std::fs::remove_dir_all(&dir);
        }
    }
}
