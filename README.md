# how-query-engines-work-rust

A faithful Rust port of the `kquery` query engine from Andy Grove's book
*How Query Engines Work*. The upstream engine is written in Kotlin; this
workspace translates it module-for-module into idiomatic Rust on top of
[Apache Arrow](https://github.com/apache/arrow-rs).

## Status

Work in progress. The workspace is fully scaffolded — every upstream Kotlin
source file has a matching Rust module — and modules are being ported
bottom-up. `datatypes` and `datasource` are ported, with passing tests.

## Workspace layout

The engine is split into the same modules as the upstream Kotlin project:

| Crate | Role |
|---|---|
| `datatypes` | Schema, fields, column vectors, record batches |
| `datasource` | `DataSource` trait + CSV / Parquet / in-memory readers |
| `logical-plan` | Logical plans, expressions, and the DataFrame builder |
| `sql` | Tokenizer, Pratt parser, and SQL-to-logical-plan planner |
| `optimizer` | Rule-based logical optimizer (projection push-down) |
| `physical-plan` | Physical operators and expressions |
| `query-planner` | Logical-to-physical plan translation |
| `execution` | Execution context and parallel execution |
| `protobuf` | Plan serialization (prost-generated types) |
| `flight-server` | Arrow Flight server |
| `client` | Arrow Flight client |
| `distributed` | Distributed scheduler and planner |
| `fuzzer` | Random query/data generator for differential testing |
| `benchmarks` | NYC-taxi and TPC-H benchmark binaries |
| `examples` | Runnable example binaries |

Shared Protocol Buffers definitions live in `proto/`; small test fixtures
live in `testdata/`.

## Building

### Prerequisites

In addition to a Rust 1.85+ toolchain (the workspace targets edition 2024),
the `protobuf` crate's `build.rs` shells out to **`protoc`** at build time to
compile the wire-format definitions in `proto/rquery.proto`. Install it once:

```bash
# macOS
brew install protobuf

# Debian / Ubuntu
sudo apt install protobuf-compiler
```

A missing `protoc` surfaces as a clear error from `tonic-build` on the first
`cargo build` that exercises the `protobuf` crate (or anything that depends
on it).

### Build & test

```bash
cargo build --workspace
cargo test --workspace
```

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE).
