//! Binary entry point for the Flight server. Ports the `main()` from
//! `kquery/flight-server/src/main/kotlin/io/FlightServer.kt`.
//!
//! All real logic lives in the library module `flight_server::serve`; this
//! file is the runnable shim that wires it up. Mirrors the Kotlin
//! single-file structure (class + `main()` in the same file) as closely
//! as Rust's lib/bin split allows.
//!
//! Defaults to listening on 0.0.0.0:50051, matching the kquery upstream
//! `:flight-server:run` Gradle target.
//!
//! ## Translation note — `#[tokio::main]`
//! Kotlin's Flight server runs on a dedicated thread launched by the JVM
//! Flight library. The Rust port uses a tokio multi-thread runtime
//! (tonic's gRPC layer requires it). `#[tokio::main]` is purely the runtime
//! launcher; the actual work happens inside `flight_server::serve`.

use flight_server::flight_server::serve;
use physical_plan::ExecutorContext;
use std::net::SocketAddr;
use tracing::error;
use tracing_subscriber::EnvFilter;

/// Default bind address — matches kquery's upstream `:flight-server:run`
/// Gradle target, which binds `0.0.0.0:50051`.
const DEFAULT_ADDR: &str = "0.0.0.0:50051";

/// Default per-executor identity and shuffle directory. Matches kquery's
/// `KQueryFlightProducer` constructor defaults
/// (`executorId = "executor-0"`, `executorHost = "localhost"`,
/// `executorPort = 50051`, `shuffleDir = "/tmp/kquery-shuffle"`) modulo the
/// project-rename of the shuffle directory. A production deployment would
/// read these from CLI / env / config; the bin keeps them inline as the
/// faithful single-file-shim shape.
const DEFAULT_EXECUTOR_ID: &str = "executor-0";
const DEFAULT_EXECUTOR_HOST: &str = "localhost";
const DEFAULT_EXECUTOR_PORT: i32 = 50051;
const DEFAULT_SHUFFLE_DIR: &str = "/tmp/rquery-shuffle";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // `RUST_LOG=info cargo run -p flight-server` enables info-level output.
    // Default level is `error` so the binary stays quiet under load unless
    // explicitly asked for more.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("error")),
        )
        .init();

    let addr: SocketAddr = DEFAULT_ADDR.parse()?;
    let ctx = ExecutorContext::new(
        DEFAULT_EXECUTOR_ID,
        DEFAULT_EXECUTOR_HOST,
        DEFAULT_EXECUTOR_PORT,
        DEFAULT_SHUFFLE_DIR,
    );

    if let Err(e) = serve(addr, ctx).await {
        error!("Flight server exited with error: {}", e);
        return Err(e.into());
    }
    Ok(())
}
