//! # fuzzer
//!
//! Random SQL / logical plan generator. Used for differential testing against
//! a reference engine.
//!
//! ## Kotlin source
//! Faithful port of `kquery/fuzzer/src/main/kotlin/Fuzzer.kt`.
//!
//! ## Status
//! TODO — module 9 of 15. Optional for the Phase-1 definition of done.

// ==============================================================
// Per-file modules — one for each upstream Kotlin source file.
// ==============================================================
pub mod fuzzer;
