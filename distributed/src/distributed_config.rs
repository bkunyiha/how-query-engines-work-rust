//! Port of `kquery/distributed/src/main/kotlin/DistributedConfig.kt`.
//!
//! `ExecutorConfig` + `DistributedConfig` data classes. Pure data, no logic
//! beyond `partition_count()` (the `getPartitionCount()` method in Kotlin).

/// Configuration for a single executor in the distributed cluster.
///
/// Kotlin: `data class ExecutorConfig(val id: String, val host: String, val port: Int)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutorConfig {
    /// Unique identifier for this executor.
    pub id: String,
    /// Hostname or IP address.
    pub host: String,
    /// Port for the Arrow Flight service.
    pub port: i32,
}

impl ExecutorConfig {
    pub fn new(id: impl Into<String>, host: impl Into<String>, port: i32) -> Self {
        Self {
            id: id.into(),
            host: host.into(),
            port,
        }
    }
}

/// Configuration for the distributed query execution environment.
///
/// Kotlin: `data class DistributedConfig(executors, shuffleDir, defaultPartitions)`.
/// Kotlin's parameter defaults (`shuffleDir = "/tmp/kquery-shuffle"`,
/// `defaultPartitions = 0`) become Rust builder methods on this struct.
///
/// ## Translation note — shuffle directory
/// The default shuffle directory is `/tmp/rquery-shuffle` (renamed from
/// kquery's `/tmp/kquery-shuffle` during the kquery → rquery user-visible-string
/// sweep — see TRANSLATION_NOTES.md, Convention section).
#[derive(Debug, Clone)]
pub struct DistributedConfig {
    /// List of executors in the cluster.
    pub executors: Vec<ExecutorConfig>,
    /// Base directory for shuffle data on each executor.
    pub shuffle_dir: String,
    /// Default number of partitions for shuffle operations. `0` is a sentinel
    /// meaning "use the number of executors" — see [`Self::partition_count`].
    pub default_partitions: i32,
}

impl DistributedConfig {
    /// Default shuffle directory. Renamed from kquery's `/tmp/kquery-shuffle`.
    pub const DEFAULT_SHUFFLE_DIR: &str = "/tmp/rquery-shuffle";

    /// Construct with sensible defaults — empty `shuffle_dir`,
    /// `default_partitions = 0` (i.e., use executor count).
    pub fn new(executors: Vec<ExecutorConfig>) -> Self {
        Self {
            executors,
            shuffle_dir: Self::DEFAULT_SHUFFLE_DIR.to_string(),
            default_partitions: 0,
        }
    }

    /// Builder: override the default shuffle directory.
    pub fn with_shuffle_dir(mut self, dir: impl Into<String>) -> Self {
        self.shuffle_dir = dir.into();
        self
    }

    /// Builder: override the default partition count. Pass `0` to fall back to
    /// the executor count.
    pub fn with_default_partitions(mut self, n: i32) -> Self {
        self.default_partitions = n;
        self
    }

    /// Effective partition count for shuffle operations.
    /// Kotlin `fun getPartitionCount(): Int`: `defaultPartitions > 0` overrides;
    /// otherwise the executor count is used.
    pub fn partition_count(&self) -> i32 {
        if self.default_partitions > 0 {
            self.default_partitions
        } else {
            self.executors.len() as i32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partition_count_defaults_to_executor_count() {
        let cfg = DistributedConfig::new(vec![
            ExecutorConfig::new("exec-1", "localhost", 50051),
            ExecutorConfig::new("exec-2", "localhost", 50052),
            ExecutorConfig::new("exec-3", "localhost", 50053),
        ]);
        assert_eq!(cfg.partition_count(), 3);
    }

    #[test]
    fn explicit_default_partitions_overrides_executor_count() {
        let cfg = DistributedConfig::new(vec![ExecutorConfig::new("exec-1", "localhost", 50051)])
            .with_default_partitions(8);
        assert_eq!(cfg.partition_count(), 8);
    }

    #[test]
    fn default_shuffle_dir_is_rquery_path() {
        let cfg = DistributedConfig::new(vec![]);
        assert_eq!(cfg.shuffle_dir, "/tmp/rquery-shuffle");
    }
}
