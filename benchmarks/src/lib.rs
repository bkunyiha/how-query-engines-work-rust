//! # benchmarks
//!
//! TPC-H runner and aggregated NYC-taxi benchmark. Ports
//! `kquery/benchmarks/src/main/kotlin/`: `Benchmarks.kt`, `TpchRunner.kt`.
//!
//! ## Definition-of-done relevance
//! Per Plan §3.9, `cargo run --bin tpch_runner -- q01.sql tpch_data/` must
//! produce results matching the Kotlin reference.
