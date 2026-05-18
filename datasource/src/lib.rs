//! # datasource
//!
//! `DataSource` trait and concrete implementations.
//!
//! ## Kotlin source
//! Faithful port of `kquery/datasource/src/main/kotlin/`:
//! `DataSource.kt`, `CsvDataSource.kt`, `InMemoryDataSource.kt`, `ParquetDataSource.kt`.
//!
//! ## Design
//! - `DataSource` is a trait; concrete impls are zero-state structs that hold paths/handles.
//! - Schema inference is eager in Phase 1 (the Kotlin code does it lazily, but eager is simpler to port).
//! - CSV reader uses arrow-rs's built-in CSV reader; Parquet reader uses the `parquet` crate.
//!
//! ## Status
//! TODO — module 2 of 15.

// ==============================================================
// Per-file modules — one for each upstream Kotlin source file.
// ==============================================================
pub mod csv_data_source;
pub mod data_source;
pub mod in_memory_data_source;
pub mod parquet_data_source;
