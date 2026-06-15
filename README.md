# rquery

[![CI](https://github.com/bkunyiha/how-query-engines-work-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/bkunyiha/how-query-engines-work-rust/actions/workflows/ci.yml)

A SQL query engine in Rust, built on [Apache Arrow](https://github.com/apache/arrow-rs).
The codebase is the companion implementation to Andy Grove's [*How Query Engines Work*](https://howqueryengineswork.com/) — every layer is small enough to read end-to-end, and the
workspace is structured so a reader can start at any module and follow
the call chain across the rest. If you are reading the book, you can
open the matching crate alongside each chapter and trace the
implementation as you go; see the [Chapter map](#chapter-map) below.

## Why Rust?

The analytical-database world has shifted to Rust, and the move has
structural reasons anyone who has tuned a high-throughput service on
the JVM will recognise. A garbage-collected runtime cannot offer the
predictable sub-millisecond latencies modern analytical workloads
demand. Object headers inflate every row of data by a fixed cost the
query planner cannot see. The JIT's warmup tax penalises the first
query of every new shape. And SIMD intrinsics, the hardware feature
that gives columnar engines their throughput, are awkward to reach
from a managed runtime. Rust addresses all four at the language
level: no garbage collector, no object headers, the same optimised
machine code on the first call and the millionth, and a first-class
portable SIMD module that compiles natively for x86-64 with AVX-2 /
AVX-512 and for ARM with NEON or SVE from a single source.

The migration is visible across the industry. The previous generation
of big-data infrastructure was almost entirely written in JVM
languages: Apache Hadoop, Apache Spark, Apache Flink, Apache
Cassandra, Apache Kafka, Apache Druid, Apache HBase. The next
generation, almost without exception, is Rust. Apache DataFusion, the
embeddable SQL query engine that descends directly from the ideas in
this book, graduated to a Top-Level Apache project in 2024 and now
sits underneath InfluxDB 3, GreptimeDB, Databend, RisingWave, dbt's
Fusion engine, Cloudflare's R2 SQL, and more than a hundred other
production analytical systems. Apache Arrow, the columnar in-memory
format underneath them all, has more contributors to its Rust
implementation alone than to every other language implementation
combined. Even Apache Spark itself is being accelerated by Rust:
Apple's DataFusion Comet intercepts a Spark physical plan and executes
supported subtrees natively in DataFusion, delivering roughly a 2×
speedup on TPC-H at the terabyte scale while passing 97% of the Spark
SQL test suite as of 2025 — the user keeps Spark, the runtime gets
faster underneath. The streaming layer has followed the same
trajectory: Arroyo, founded by a former Apache Flink lead, rebuilt
its engine around Arrow and DataFusion in 2024 and reported 3× higher
throughput and 20× faster startup on the same workload, and an
independent 2026 benchmark comparing a Rust streaming pipeline against
Apache Spark Structured Streaming on a 9.89-million-row financial
workload found the Rust pipeline roughly 1.6× faster end-to-end, with
per-batch latency 7× lower, transform time 9× lower, and Delta Lake
write time 18× lower than Spark's. The acronym people use for this
convergence — **FDAP**: Flight, DataFusion, Arrow, Parquet —
describes a Rust-native pipeline from disk through the network to the
executor with no copies in between.

This codebase exists as a way into that stack. It's a faithful Rust
port of the Kotlin query engine described in Andy Grove's [*How Query Engines Work*](https://howqueryengineswork.com/) — the same architecture, the same operators, the same
distributed-execution shape, in the language the analytical world is
now writing in.

## Chapter map

The crates in this workspace correspond to the chapters of [*How Query Engines Work*](https://howqueryengineswork.com/). As you read each chapter, open the listed crate or file
alongside it — the code is laid out so the chapter and the source can
be read in parallel.

| Chapter | Title | Crate(s) in this repo |
|---|---|---|
| 1 | What Is a Query Engine? | (intro; no code substitution) |
| 2 | Apache Arrow | (conceptual; underpins every crate) |
| 3 | Type System | `datatypes` |
| 4 | Data Sources | `datasource` |
| 5 | Logical Plans and Expressions | `logical-plan` |
| 6 | DataFrame API | `logical-plan/src/data_frame.rs` |
| 7 | SQL Support | `sql` |
| 8 | Physical Plans and Expressions | `physical-plan` |
| 9 | Query Planner | `query-planner` |
| 10 | Joins | `physical-plan/src/hash_join_exec.rs` + `logical-plan/src/join.rs` |
| 11 | Subqueries | (conceptual; not implemented) |
| 12 | Query Optimizations | `optimizer` |
| 13 | Query Execution | `execution/src/execution_context.rs` |
| 14 | Parallel Query Execution | `execution/src/parallel_context.rs` |
| 15 | Distributed Query Execution | `distributed` + `flight-server` + `client` |
| 16 | Testing | `fuzzer` (plus `#[cfg(test)]` blocks across the workspace) |
| 17 | Benchmarks | `benchmarks` |

## What it does

Run a SQL query against a CSV or Parquet file:

```rust
use execution::ExecutionContext;
use std::collections::HashMap;

let mut ctx = ExecutionContext::new(HashMap::new());
ctx.register_csv("employee", "testdata/employee.csv");
let results: Vec<_> = ctx
    .sql("SELECT state, SUM(salary) FROM employee GROUP BY state")
    .collect();
```

Or run the same query distributed across an Arrow Flight cluster:

```rust
use distributed::{DistributedConfig, DistributedContext, ExecutorConfig};
use client::FlightExecutorClient;

let executors = vec![ExecutorConfig::new("exec-1", "127.0.0.1", 50051)];
let flight_client = FlightExecutorClient::new(&executors)?;
let config = DistributedConfig::new(executors).with_default_partitions(3);
let mut ctx = DistributedContext::new(config, flight_client);
ctx.register_csv("employee", "testdata/employee.csv", true);
let results: Vec<_> = ctx
    .sql("SELECT state, SUM(salary) FROM employee GROUP BY state")
    .collect();
```

Single-process or distributed, the same SQL → logical plan → physical
plan pipeline produces the answer.

## Workspace layout

The engine is split into 15 small crates:

| Crate | Role |
|---|---|
| `datatypes` | `Schema`, `Field`, `ColumnVector`, `RecordBatch` wrappers over arrow-rs |
| `datasource` | `DataSource` trait plus CSV, Parquet, and in-memory implementations |
| `logical-plan` | `LogicalPlan` enum, `LogicalExpr` expressions, and the `DataFrame` builder |
| `sql` | Tokenizer, Pratt parser, and SQL-to-logical-plan compiler |
| `optimizer` | Rule-based logical optimizer (currently: projection push-down) |
| `physical-plan` | Physical operators (`ScanExec`, `HashAggregateExec`, etc.) and expressions |
| `query-planner` | Lowers `LogicalPlan` → `Arc<dyn PhysicalPlan>` |
| `execution` | `ExecutionContext` (sequential) and `ParallelContext` (rayon) |
| `protobuf` | Wire-format types generated from `proto/rquery.proto` via `prost-build` |
| `flight-server` | Arrow Flight server exposing the engine over gRPC |
| `client` | Sync Arrow Flight client and `FlightExecutorClient` for distributed |
| `distributed` | Distributed scheduler, planner, and `DistributedContext` |
| `fuzzer` | Random query/data generator for differential testing |
| `benchmarks` | NYC-taxi and TPC-H benchmark binaries |
| `examples` | Runnable example binaries (see below) |

Shared Protocol Buffers definitions live in `proto/`; test fixtures live in
`testdata/`.

## Building

### Prerequisites

You'll need a Rust 1.85+ toolchain (the workspace targets edition 2024).

The `protobuf` crate's `build.rs` shells out to **`protoc`** at build time
to compile the wire-format definitions in `proto/rquery.proto`. Install it
once:

```bash
# macOS
brew install protobuf

# Debian / Ubuntu
sudo apt install protobuf-compiler
```

A missing `protoc` surfaces as a clear error from `tonic-build` on the
first `cargo build` that exercises the `protobuf` crate (or anything that
depends on it).

### Build and test

```bash
cargo build --workspace
cargo test --workspace
```

### Optional: clippy and rustfmt

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

## Running the examples

The `examples/` crate ships five runnable binaries. Run from the
`examples/` directory so the relative path to `testdata/` resolves
correctly:

```bash
cd examples
cargo run --bin parallel_execution_example   # sequential vs parallel side-by-side
cargo run --bin distributed_example          # single-process distributed-shape demo
cargo run --bin distributed_flight_example   # real Arrow Flight gRPC distributed query
cargo run --bin nyc_taxi                     # NYC yellow-taxi 2019-12 benchmark
cargo run --bin parallel_query               # 12 monthly NYC-taxi queries in parallel
```

The first three operate on the in-repo `testdata/employee.csv` and need
no external downloads. `nyc_taxi` and `parallel_query` expect the NYC
yellow-taxi 2019 dataset to be present at the hardcoded path; see each
file's module doc for details.

## Reading order

If you are reading the book, the [Chapter map](#chapter-map) above is
the natural path — each chapter pairs with a specific crate or file.

If you are reading the source independently of the book, the natural
order matches the data flow through the engine — which also matches
the book's chapter sequence, so either entry point ends up in the same
place:

1. **`datatypes/`** — `Schema`, `RecordBatch`, `ColumnVector`. The
   data-shape layer.
2. **`datasource/`** — How CSV and Parquet files become `RecordBatch`
   streams.
3. **`logical-plan/`** — The `LogicalPlan` enum, `LogicalExpr`, and the
   `DataFrame` builder.
4. **`sql/`** — Tokenize, Pratt-parse, lower into `DataFrame`.
5. **`optimizer/`** — `ProjectionPushDownRule`.
6. **`physical-plan/`** — `PhysicalPlan` trait, every operator, every
   expression. The largest module.
7. **`query-planner/`** — Lowers a `LogicalPlan` into a tree of
   `Arc<dyn PhysicalPlan>`.
8. **`execution/`** — `ExecutionContext::sql("SELECT ...").collect()`.
9. **`distributed/`** — `Scheduler`, `DistributedPlanner`,
   `DistributedContext`. The single-machine engine, but planned and
   dispatched as if it were a cluster.
10. **`flight-server/`** and **`client/`** — Make the cluster real:
    Arrow Flight over tonic gRPC.

## License

Apache License 2.0. See [LICENSE](LICENSE).
