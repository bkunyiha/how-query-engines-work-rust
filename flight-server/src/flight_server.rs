//! Port of `kquery/flight-server/src/main/kotlin/io/FlightServer.kt`.
//!
//! Contains the `FlightServer` struct and its associated logic. The runnable
//! entry point (the Kotlin `main()`) lives in `src/bin/flight_server.rs`,
//! which calls into this module.
//!
//! TODO: port from upstream Kotlin. See [`ARCHITECTURE.md`] §4 for the per-module
//! porting plan and §3 for the Kotlin → Rust idiom cheatsheet.
//!
//! [`ARCHITECTURE.md`]: ../../ARCHITECTURE.md
