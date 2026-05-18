//! # protobuf
//!
//! Protocol Buffers schemas + serialise / deserialise helpers for
//! `PhysicalPlan`, `LogicalExpr`, `Action`, and the related shuffle types.
//!
//! ## Kotlin source
//! Faithful port of `kquery/protobuf/src/main/kotlin/`:
//! `PhysicalPlanSerializer.kt`, `PhysicalPlanDeserializer.kt`,
//! `ProtobufSerializer.kt`, `ProtobufDeserializer.kt`.
//!
//! The `.proto` schemas live under `../../proto/`. Code generation is driven
//! by `build.rs` via `tonic-build` (which wraps `prost-build`).
//!
//! ## Status
//! TODO — module 12 of 15.

// ==============================================================
// Per-file modules — one for each upstream Kotlin source file.
// ==============================================================
pub mod physical_plan_deserializer;
pub mod physical_plan_serializer;
pub mod protobuf_deserializer;
pub mod protobuf_serializer;
