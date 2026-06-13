//! # client
//!
//! Two roles in one crate:
//!
//! 1. **Interactive Flight client** — [`Client`](client::Client) and
//!    [`Context`](context::Context) wrap an Arrow Flight connection so a user
//!    can submit a `LogicalPlan` and receive `RecordBatch`es back. Faithful
//!    port of `kquery/client/`'s `Client.kt` and `Context.kt`.
//! 2. **Distributed-scheduler transport** —
//!    [`FlightExecutorClient`](flight_executor_client::FlightExecutorClient)
//!    implements `distributed::ExecutorClient` over a set of per-executor
//!    `Client` instances, so `Scheduler<FlightExecutorClient>::execute(...)`
//!    drives a real distributed query against real Flight executors. **Not
//!    present in kquery** — a Phase-1 rquery addition that closes the §7
//!    mock rows in `DISTRIBUTED_MODULE_WALKTHROUGH.md`. Documented as a
//!    divergence in `TRANSLATION_NOTES.md` → Module: client.
//!
//! ## Kotlin source
//! - `kquery/client/src/main/kotlin/Client.kt` → `src/client.rs`
//! - `kquery/client/src/main/kotlin/Context.kt` → `src/context.rs`
//! - (no Kotlin counterpart) → `src/endpoint.rs` (bundles host+port into one
//!   value; `Client.kt` takes them as separate ctor args)
//! - (no Kotlin counterpart) → `src/flight_executor_client.rs` (rquery
//!   addition; see above)
//!
//! ## Module 14 — porting status (this file)
//!
//! | Batch | Scope                                                                | Status |
//! |-------|----------------------------------------------------------------------|--------|
//! | A     | crate skeleton + Cargo.toml deps + module stubs                      | **done (this commit)** |
//! | B     | `Endpoint` data type + `Client::new` (connect to one server)         | pending |
//! | C     | `Client::do_action` / `Client::do_get` (sync wrappers over async tonic) | pending |
//! | D     | `Context` — mirrors `ExecutionContext` API, routes through `Client::execute` | pending |
//! | E     | `FlightExecutorClient` — `impl distributed::ExecutorClient`          | pending |
//! | F     | Integration test against in-process flight-server                    | pending |
//! | G     | TRANSLATION_NOTES.md entries                                         | pending |
//! | H     | `cargo build --workspace && cargo test --workspace`                  | pending |

// ==============================================================
// Per-file modules.
// ==============================================================
pub mod client;
pub mod context;
pub mod endpoint;
pub mod flight_executor_client;

// ==============================================================
// Re-exports for convenient downstream `use client::FlightExecutorClient;`.
// ==============================================================
pub use client::Client;
pub use context::Context;
pub use endpoint::Endpoint;
pub use flight_executor_client::FlightExecutorClient;
