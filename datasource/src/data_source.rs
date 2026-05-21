//! Port of `kquery/datasource/src/main/kotlin/DataSource.kt`.
//!
//! Trait that every concrete data source implements.
//!
//! Translation notes:
//! - Kotlin `interface DataSource` → Rust `pub trait DataSource`.
//! - Kotlin `fun scan(projection: List<String>): Sequence<RecordBatch>` returns
//!   a lazy stream of batches. `Sequence<T>` becomes
//!   `Box<dyn Iterator<Item = T>>` in this port. Errors panic rather
//!   than being threaded through `Result<_>` — a future Rustified rewrite will
//!   switch to `Stream<Item = Result<RecordBatch, FdapError>>`.

use datatypes::{RecordBatch, Schema};

/// Trait for any source that can describe its schema and produce batches.
pub trait DataSource {
    /// Return the schema for the underlying data source.
    fn schema(&self) -> Schema;

    /// Scan the data source, selecting the specified columns. An empty
    /// `projection` slice means "all columns".
    fn scan(&self, projection: &[String]) -> Box<dyn Iterator<Item = RecordBatch>>;
}
