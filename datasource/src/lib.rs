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
//! - `CsvDataSource` uses arrow-rs's built-in `arrow::csv::ReaderBuilder` instead of porting
//!   Andy's hand-rolled univocity-parsers logic.
//! - `ParquetDataSource` uses arrow-rs's built-in `parquet::arrow::ParquetRecordBatchReaderBuilder`
//!   instead of porting the Hadoop / parquet-arrow / GroupRecordConverter dispatch. Same rationale.
//! - Schema inference in CSV uses arrow-rs's typed inference (Int64, Float64, Boolean, Utf8)
//!   rather than the Kotlin original's everything-as-String inference.
//!
//! ## Status
//! Module 2 of 15 — ported. All 4 Kotlin source files have Rust equivalents.

// ==============================================================
// Per-file modules — one for each upstream Kotlin source file.
// ==============================================================
pub mod csv_data_source;
pub mod data_source;
pub mod in_memory_data_source;
pub mod parquet_data_source;

// ==============================================================
// Re-exports for convenient downstream `use datasource::*;` ergonomics.
// ==============================================================
pub use csv_data_source::CsvDataSource;
pub use data_source::DataSource;
pub use in_memory_data_source::InMemoryDataSource;
pub use parquet_data_source::ParquetDataSource;
