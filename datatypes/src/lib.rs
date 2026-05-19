//! # datatypes
//!
//! Core data types — the substrate every other crate builds on.
//!
//! ## Kotlin source
//! Faithful port of `kquery/datatypes/src/main/kotlin/`:
//! `ArrowFieldVector.kt`, `ArrowTypes.kt`, `ArrowVectorBuilder.kt`,
//! `ColumnVector.kt`, `LiteralValueVector.kt`, `RecordBatch.kt`,
//! `Schema.kt`, `ShuffleId.kt`, `ShuffleLocation.kt`.
//!
//! ## Design
//! - Interface hierarchies in Kotlin become Rust `enum`s.
//! - `ColumnVector` is a trait; concrete impls wrap arrow-rs arrays.
//! - `Schema` and `Field` are `#[derive(Clone, Debug, PartialEq)]` structs.
//! - **`ScalarValue`** is added in the Rust port (no Kotlin counterpart) to
//!   replace the Kotlin `Any?` return type of `ColumnVector.getValue`.
//!   It is the *only* place this module introduces
//!   structure that wasn't in the Kotlin original.
//!
//! ## Status
//! Module 1 of 15 — ported. All 9 Kotlin source files have Rust equivalents,
//! plus the added `scalar_value.rs`. Deliberate divergences: Rayon for
//! coroutines, arrow-rs `RecordBatch` re-export, no `ArrowAllocator`, and the
//! builder API shape change.

// ==============================================================
// Per-file modules — one for each upstream Kotlin source file,
// plus `scalar_value` (added to replace Kotlin's `Any?` with a typed enum).
// ==============================================================
pub mod arrow_field_vector;
pub mod arrow_types;
pub mod arrow_vector_builder;
pub mod column_vector;
pub mod literal_value_vector;
pub mod record_batch;
pub mod scalar_value;
pub mod schema;
pub mod shuffle_id;
pub mod shuffle_location;

// ==============================================================
// Re-exports for convenient downstream `use datatypes::*;` ergonomics.
// ==============================================================
pub use arrow_field_vector::ArrowFieldVector;
pub use arrow_vector_builder::ArrowVectorBuilder;
pub use column_vector::ColumnVector;
pub use literal_value_vector::LiteralValueVector;
pub use record_batch::RecordBatch;
pub use scalar_value::ScalarValue;
pub use schema::{Field, Schema, SchemaConverter};
pub use shuffle_id::ShuffleId;
pub use shuffle_location::ShuffleLocation;
