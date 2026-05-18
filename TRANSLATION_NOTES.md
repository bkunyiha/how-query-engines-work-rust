# Translation Notes — Kotlin `kquery` → Rust `rquery`

This file documents every place the Rust port deliberately diverges from the Kotlin source.

The [`ARCHITECTURE.md`](ARCHITECTURE.md) §3 cheatsheet covers the *standard* substitutions (`sealed class` → `enum`, `data class` → `struct`, `throw` → `panic!`, Kotlin `when` → Rust `match`, etc.) — those do not need entries here. This file captures the *non-standard* ones: deviations from the default translation strategy. Examples of what belongs here include library-forced substitutions (e.g., using arrow-rs's `csv::ReaderBuilder` instead of hand-rolling the CSV parser kquery implements in `CsvDataSource.kt`), ecosystem-forced substitutions (e.g., the Flight server going async because `tonic` requires it), CPU-parallelism substitutions (the three Kotlin `runBlocking { async { } }` sites becoming Rayon), and any other place where the Rust code does not mirror the Kotlin shape line-for-line.

Required by the port's [definition of done](README.md#definition-of-done).

## Convention

One entry per deliberate deviation. Each entry contains:

- **Date** in ISO format (`yyyy-mm-dd`).
- **Subject** — the Kotlin construct, file, or behaviour that was not directly translated.
- **Substitution** — the Rust replacement, in one sentence.
- **Rationale** — the reason the standard translation didn't apply.
- **Cross-reference** — the relevant section of the Translation Plan, where applicable.

Entries are grouped by module (matching the §4 module-by-module porting plan). Within a module, entries are listed in date order, oldest first.

**Update this file in the same commit as the diverging code.** Batching divergences for a "cleanup commit later" never works — by the time you come back, the rationale is gone. This is the single rule that determines whether the file remains a useful audit trail or rots into stale prose.

---

## Per-Module Log

### Module: datatypes

*No entries yet.*

### Module: datasource

*No entries yet.*

### Module: logical-plan

*No entries yet.*

### Module: sql

*No entries yet.*

### Module: optimizer

*No entries yet.*

### Module: physical-plan

*No entries yet.*

### Module: query-planner

*No entries yet.*

### Module: execution

*No entries yet.*

### Module: fuzzer

*No entries yet.*

### Module: examples

*No entries yet.*

### Module: benchmarks

*No entries yet.*

### Module: protobuf

*No entries yet.*

### Module: flight-server

*No entries yet.*

### Module: client

*No entries yet.*

### Module: distributed

*No entries yet.*

---

> *Every entry above is one place a future reviewer should be able to ask "why?" and get a precise answer pointing back to the planning rationale or the dated decision. If an entry can't survive that test, expand it or remove it.*
