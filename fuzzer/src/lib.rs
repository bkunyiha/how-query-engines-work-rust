//! # fuzzer
//!
//! Random SQL / logical plan generator. Used for differential testing against
//! a reference engine.
//!
//! ## Kotlin source
//! Faithful port of `kquery/fuzzer/src/main/kotlin/Fuzzer.kt`.
//!
//! ## Status
//! Module 9 of 15 — **ported**. Unblocks the `Fuzzer`-backed `ExecutionTest`
//! cases that were skipped pending this module.

// ==============================================================
// Per-file modules — one for each upstream Kotlin source file.
// ==============================================================
pub mod fuzzer;

// Re-export the public surface so callers can write `fuzzer::Fuzzer` rather
// than `fuzzer::fuzzer::Fuzzer`.
pub use fuzzer::{EnhancedRandom, Fuzzer};
