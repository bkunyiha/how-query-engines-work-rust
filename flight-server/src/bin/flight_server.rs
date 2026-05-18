//! Binary entry point for the Flight server. Ports the `main()` from
//! `kquery/flight-server/src/main/kotlin/io/FlightServer.kt`.
//!
//! All real logic lives in the library module `flight_server.rs`; this
//! file is the runnable shim that wires it up. Mirrors the Kotlin
//! single-file structure (class + `main()` in the same file) as closely
//! as Rust's lib/bin split allows.
//!
//! Defaults to listening on 0.0.0.0:50051, matching the kquery upstream
//! `:flight-server:run` Gradle target.

fn main() {
    env_logger::init();
    todo!("port the main() from kquery/flight-server/.../FlightServer.kt; \
           delegate to the FlightServer struct in src/flight_server.rs")
}
