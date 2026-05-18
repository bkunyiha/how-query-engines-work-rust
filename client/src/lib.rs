//! # client
//!
//! Client wrapper around an Arrow Flight connection. Serialises a logical
//! plan to protobuf, calls the server, receives Arrow RecordBatches back.
//!
//! ## Kotlin source
//! Faithful port of `kquery/client/src/main/kotlin/`:
//! `Client.kt`, `Context.kt`.
//!
//! ## Status
//! TODO — module 14 of 15.

// ==============================================================
// Per-file modules — one for each upstream Kotlin source file.
// ==============================================================
pub mod client;
pub mod context;
