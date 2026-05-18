//! # datatypes
//!
//! Core data types — the substrate every other crate builds on.
//!
//! ## Kotlin source
//! Faithful port of `kquery/datatypes/src/main/kotlin/`:
//! `ArrowFieldVector.kt`, `ArrowTypes.kt`, `ArrowVectorBuilder.kt`,
//! `ColumnVector.kt`, `LiteralValueVector.kt`, `RecordBatch.kt`,
//! `Schema.kt`, `Field.kt`, and the related helpers.
//!
//! ## Design
//! - Sealed-class hierarchies in Kotlin become Rust `enum`s (see Plan §3.4).
//! - `ColumnVector` is a trait; concrete impls wrap arrow-rs arrays.
//! - `Schema` and `Field` are `#[derive(Clone, Debug, PartialEq)]` structs.
//!
//! ## Status
//! TODO — module 1 of 15. Build first per Plan §3.5.

// ==============================================================
// Per-file modules — one for each upstream Kotlin source file.
// ==============================================================
pub mod arrow_field_vector;
pub mod arrow_types;
pub mod arrow_vector_builder;
pub mod column_vector;
pub mod literal_value_vector;
pub mod record_batch;
pub mod schema;
pub mod shuffle_id;
pub mod shuffle_location;
