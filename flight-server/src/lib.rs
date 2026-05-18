//! # flight-server
//!
//! Arrow Flight server that exposes the query engine over gRPC.
//!
//! ## Kotlin source
//! Faithful port of `kquery/flight-server/src/main/kotlin/io/`:
//! `FlightServer.kt`, `KQueryFlightProducer.kt`.
//!
//! ## Status
//! TODO — module 13 of 15.

// ==============================================================
// Per-file modules — one for each upstream Kotlin source file.
// `KQueryFlightProducer.kt` rebrands to `r_query_flight_producer.rs`
// per the §3.0 project-name-prefix rule (K for Kotlin → R for Rust).
// ==============================================================
pub mod r_query_flight_producer;
pub mod flight_server;
