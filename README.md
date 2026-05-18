# rquery — Faithful Rust Port of `kquery`

A line-by-line Rust translation of [Andy Grove's `kquery`](https://github.com/andygrove/how-query-engines-work) — the Kotlin query engine from *How Query Engines Work* (2nd edition).

The port is deliberately *faithful*: same type names, same method names, same algorithms — translated, not redesigned. Performance, async, production-quality error handling, and arrow-rs interop beyond the bare minimum are explicit *non-goals*. The intent is to internalise the engine's design by mirroring it in Rust, not to improve on it.

A future second pass — a Rustified rewrite against arrow-rs, Tokio, `anyhow`/`thiserror`, and FlightSQL — is planned as a separate workspace once this port is complete.

## Why faithful first

A port like this decouples two learning curves that are fatal to attempt in parallel:

1. *Query engine internals* — what a logical plan is, why expressions are ASTs, how a hash aggregate works, what shuffle means.
2. *The Rust analytical-database runtime* — `arrow-rs` ergonomics, parquet async readers, Tokio streams, lifetimes inside columnar buffers.

This port isolates curve 1: faithful translation of an unoptimised reference engine, using only the parts of arrow-rs strictly required to read CSV / Parquet test data. The single discipline that makes this work is *resisting the urge to redesign*. Every refactor postponed is a refactor better understood after the original is ported first.

For the full porting methodology — idiom translation cheatsheet, module-by-module plan, empirical-findings table proving the port's structural claims against the upstream Kotlin source, file-naming convention, and TRANSLATION_NOTES.md workflow — see [`ARCHITECTURE.md`](ARCHITECTURE.md).

## Module map (Kotlin → Rust)

The Kotlin Gradle modules map 1:1 to Rust crates at the workspace root, matching the upstream `kquery` directory layout exactly. Build bottom-up — never start a crate until everything in its "Depends on" column is done.

| # | Crate | Kotlin module | Depends on (internal) |
|---|---|---|---|
| 1 | `datatypes` | `datatypes` | — |
| 2 | `datasource` | `datasource` | datatypes |
| 3 | `logical-plan` | `logical-plan` | datatypes |
| 4 | `sql` | `sql` | logical-plan, datatypes |
| 5 | `optimizer` | `optimizer` | logical-plan |
| 6 | `physical-plan` | `physical-plan` | logical-plan, datatypes, datasource |
| 7 | `query-planner` | `query-planner` | logical-plan, physical-plan |
| 8 | `execution` | `execution` | query-planner, optimizer |
| 9 | `fuzzer` | `fuzzer` | logical-plan, sql |
| 10 | `examples` | `examples` | execution |
| 11 | `benchmarks` | `benchmarks` | execution |
| 12 | `protobuf` | `protobuf` | physical-plan |
| 13 | `flight-server` | `flight-server` | datatypes, datasource, logical-plan, physical-plan, query-planner, sql, protobuf, execution |
| 14 | `client` | `client` | datatypes, datasource, logical-plan, protobuf |
| 15 | `distributed` | `distributed` | datatypes, datasource, logical-plan, physical-plan, query-planner, optimizer, execution, sql, protobuf |

Note: `client` and `distributed` do **not** depend on `flight-server` in-process. They speak to a running Flight server over gRPC via the external `arrow-flight` crate. The `flight-server` crate has zero in-workspace consumers — it's a runnable, not a library that other crates link against. Verified against the upstream Kotlin Gradle dependency declarations; see [`ARCHITECTURE.md`](ARCHITECTURE.md) §1.4 for the empirical-findings table that documents this.

**JDBC is deliberately skipped.** Kotlin's `jdbc` module wraps the Flight server with a JVM JDBC driver, which has no direct Rust analogue. A future Rust rewrite will replace it with FlightSQL + ADBC instead.

## Layout

```
how-query-engines-work-rust/
├── Cargo.toml                  # workspace manifest
├── README.md                   # this file
├── ARCHITECTURE.md             # porting methodology and technical reference
├── TRANSLATION_NOTES.md        # audit log of deliberate divergences from the Kotlin source
├── .gitignore
├── LICENSE                     # Apache-2.0 (matches upstream kquery)
├── datatypes/                  ┐
├── datasource/                 │
├── logical-plan/               │
├── sql/                        │
├── optimizer/                  │
├── physical-plan/              ├── 15 crates, one per upstream Kotlin Gradle module.
├── query-planner/              │   Directory layout is 1:1 with upstream how-query-engines-work/.
├── execution/                  │
├── fuzzer/                     │
├── examples/                   │
├── benchmarks/                 │
├── protobuf/                   │
├── flight-server/              │
├── client/                     │
├── distributed/                ┘
├── testdata/                   # small fixtures copied verbatim from kquery/testdata
└── proto/                      # shared protobuf .proto files for the protobuf crate
```

**Per-file 1:1 inside each crate.** Beyond the directory-and-crate-name parity above, each Kotlin source file has a corresponding Rust stub file inside the matching crate, named by PascalCase → snake_case conversion with acronym preservation, plus one rebrand rule for the project-name prefix:

| Kotlin file | Rust file | Rule |
|---|---|---|
| `HashAggregateExec.kt` | `hash_aggregate_exec.rs` | mechanical |
| `LogicalPlan.kt` | `logical_plan.rs` | mechanical |
| `NYCTaxi.kt` | `nyc_taxi.rs` | acronym preserved |
| `KQueryFlightProducer.kt` | `r_query_flight_producer.rs` | **project-name rebrand** (K for Kotlin → R for Rust) |

Each stub contains a doc comment pointing back to the matching Kotlin source path. The crate's `lib.rs` declares each stub as `pub mod <snake_case>;`. Total: 83 stub files across 13 library crates plus 7 binary entries. To port a Kotlin file, open it side by side with its matching Rust stub — *where each type belongs is already decided by the file structure*. Full naming convention and per-module porting plan: [`ARCHITECTURE.md`](ARCHITECTURE.md) §3 (Idiom Cheatsheet) and §4 (Module-by-Module Plan).

## Definition of done

This port is complete when:

- `cargo build --workspace` succeeds with zero warnings
- `cargo test --workspace` passes
- `cargo run --bin nyc_taxi` produces output matching the Kotlin reference within rounding tolerance
- `cargo run --bin tpch_runner -- q01.sql tpch_data/` produces TPC-H Q1 results matching the Kotlin reference
- `TRANSLATION_NOTES.md` documents every place the Rust port deliberately diverges from the Kotlin source

## Style and translation conventions

The full Kotlin → Rust idiom cheatsheet lives in [`ARCHITECTURE.md`](ARCHITECTURE.md) §3. Key rules:

- Kotlin `sealed class` → Rust `enum`. Match exhaustively.
- Kotlin `interface` → Rust `trait`. Free functions become trait methods.
- Kotlin `when (x) { is A -> …; is B -> … }` → Rust `match x { A(_) => …, B(_) => … }`. The dominant dispatch idiom in `kquery`; translates 1:1.
- Kotlin `Sequence<T>` → Rust `Box<dyn Iterator<Item = T>>`. Synchronous in this port.
- Kotlin `throw IllegalStateException(...)` → `panic!(...)` / `.expect(...)`. Conversion to `Result<T, E>` is deferred to a future rewrite.
- Kotlin `runBlocking { async { } }` for CPU parallelism → Rust `rayon::par_iter` / `rayon::scope`. Tokio is *not* used here (the upstream code has no async I/O); see ARCHITECTURE.md §3.9 for the empirical analysis.
- `#[derive(Clone)]` aggressively. Avoid borrow-checker fights at this stage; a future rewrite will replace clones with borrows once the topology is understood.
- The Flight server module is the *one* place where `async fn` is forced — `tonic` (the Rust gRPC framework underlying `arrow-flight`) is async-only, unlike Java gRPC's synchronous stub. Wrap synchronous engine logic in `tokio::task::spawn_blocking` to keep the async cordoned to the Flight boundary.

## License

Apache-2.0, matching upstream `kquery`. See [`LICENSE`](LICENSE).

## Relationship to *How Query Engines Work*

This is a community-built Rust companion to Andy Grove's book and its Kotlin reference implementation. It is not (yet) an official Rust edition of the book; Andy has not endorsed this port. The book is at <https://leanpub.com/how-query-engines-work> and the free HTML version at <https://howqueryengineswork.com>; the Kotlin source is at <https://github.com/andygrove/how-query-engines-work>. This port stands or falls on whether reading it alongside the book makes the same concepts click for a Rust-native reader.
