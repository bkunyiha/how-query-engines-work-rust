//! Port of `kquery/flight-server/src/main/kotlin/KQueryFlightProducer.kt`.
//!
//! Rust type name: `RQueryFlightProducer` (rebranded from Kotlin's
//! `KQueryFlightProducer` per the project-name-prefix rule — "K" for
//! Kotlin → "R" for Rust). File name follows the same rule:
//! `k_query_flight_producer.rs` → `r_query_flight_producer.rs`.
//!
//! Implements `arrow_flight::flight_service_server::FlightService` and is
//! constructed by `RQueryFlightServer` in `flight_server.rs`.
//!
//! TODO: port from the upstream Kotlin source.
