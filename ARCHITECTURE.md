# rquery Architecture — Porting Methodology and Technical Reference

> Companion document to the workspace [`README.md`](README.md). This file is the operational reference for actually doing the port — the idiom-translation cheatsheet, the module-by-module porting plan, the empirical-findings table that documents what the upstream Kotlin codebase actually contains (vs. what generic "Kotlin → Rust" guides assume), and the conventions for logging deliberate divergences in [`TRANSLATION_NOTES.md`](TRANSLATION_NOTES.md).

---

## Table of Contents

1. [About This Document](#1-about-this-document) — thesis, how to read it, glossary, empirical findings
2. [Pre-Requisites and Tooling Setup](#2-pre-requisites-and-tooling-setup) — software, IDE, the `justfile` template
3. [The Kotlin → Rust Idiom Cheatsheet](#3-the-kotlin--rust-idiom-cheatsheet) — naming convention, language idioms, one-page recap
4. [The Module-by-Module Porting Plan](#4-the-module-by-module-porting-plan) — 15 crates in build order with per-module gotchas and definition of done
5. [Testing Strategy](#5-testing-strategy) — unit, snapshot, end-to-end, differential
6. [Development Workflow](#6-development-workflow) — per-module checklist, `TRANSLATION_NOTES.md`, commit conventions
7. [Common Pitfalls](#7-common-pitfalls) — the traps that derail faithful ports
8. [Definition of Done](#8-definition-of-done) — per-module hygiene + workspace-level acceptance criteria

---

## 1. About This Document

### 1.1 The Thesis

A faithful Kotlin → Rust port of a query engine is a tractable, finite project that produces a working analytical engine and, more importantly, internalises every concept a database internals engineer needs. The exercise has one rule: **resist the urge to redesign**. A second pass — a Rustified rewrite against arrow-rs, Tokio, and FlightSQL — is planned as a separate workspace once this faithful port is complete. Each pass isolates one learning curve.

Kotlin and Rust are unusually well-aligned for this kind of port. Kotlin's `sealed class` hierarchies are, per the Kotlin community itself, *"almost exactly Rust enums"* ([Kotlin Discussions, 2018](https://discuss.kotlinlang.org/t/sealed-classes-rust-enums/2461)). Kotlin's `data class` becomes a `struct` with derive macros. Kotlin's `interface` becomes a Rust `trait`. The mechanical mapping is direct enough that most files port at a rate of roughly 100 Kotlin lines of code per hour once you settle into the rhythm.

The non-mechanical parts fall into two camps:

- **Substituted in this port** (logged in `TRANSLATION_NOTES.md`): the three files using `kotlinx.coroutines` for CPU parallelism (`execution/ParallelContext.kt`, `examples/ParallelQuery.kt`, `benchmarks/Benchmarks.kt`) become Rayon — see §3.9 — plus a handful of library-forced cases like delegating CSV parsing to `arrow::csv::ReaderBuilder` instead of hand-rolling it.
- **Deferred to a future rewrite** (intentionally not yet idiomatic): JVM exceptions become `panic!()` / `.expect(...)` here and would become `Result<T, FdapError>` later; `Sequence<T>` becomes a synchronous `Iterator<Item = T>` here and would become `futures::Stream` later.

§6 below describes the workflow for capturing every divergence as it happens.

### 1.2 How to Read This Document

Two reading modes:

- **Cover-to-cover, once, before starting.** Sections in order; ~45 minutes. Read this way the first time so the shape of the port is internalised before any code is written. Pay particular attention to §3 (idiom translations) and §4 (module-by-module plan).
- **Reference lookup, repeatedly, during the port.** §3.14 (cheatsheet table) is the single-page summary you'll glance at while porting; §3.1–§3.13 expand each row with examples and caveats. §4.x is the per-module deep dive — open the entry for the module you're currently porting and keep it visible. §7 (pitfalls) is worth re-reading at the start of each new module.

### 1.3 Glossary

| Term | Meaning |
|---|---|
| **kquery** | Andy Grove's Kotlin reference implementation of the engine described in *How Query Engines Work* (2nd edition). The source of truth this port follows. Upstream at <https://github.com/andygrove/how-query-engines-work>. |
| **rquery** | The Rust workspace this document describes. The "R" stands for Rust, matching kquery's "K" for Kotlin. |
| **TPC-H** | A standard analytical-query benchmark from the Transaction Processing Performance Council. Used in §4.11 to validate end-to-end correctness; "Q1" is its first canonical query (aggregation over `lineitem` grouped by return-flag and line-status). |
| **Pratt parser** | A "top-down operator-precedence" parsing algorithm — the SQL module's parsing approach. Each token type declares a precedence and a parse function; the parser drives expression parsing recursively by precedence. Vaughan Pratt published the technique in 1973. Hand-porting it (rather than substituting `sqlparser-rs`) is the pedagogical core of the SQL module — see §4.4. |
| **ScalarValue** | A typed enum used in the Rust port wherever the Kotlin source uses `Any`. Each Arrow scalar type becomes a variant: `ScalarValue::Int64(i64)`, `ScalarValue::Utf8(String)`, etc. This trades runtime type-tags for compile-time exhaustiveness — see §3.1. |
| **DataFrame** | A fluent builder around a `LogicalPlan`. Lets you write `df.project(...).filter(...).limit(...)` instead of constructing the plan tree by hand. See §4.3. |
| **`TRANSLATION_NOTES.md`** | The audit-trail file at the workspace root that documents every place the Rust port deliberately diverges from the Kotlin source. See §6.2. |
| **`[[bin]]` entry** | A Cargo manifest declaration that exposes an additional binary target for a crate. Used in `examples/`, `benchmarks/`, and `flight-server/` where each runnable lives as its own `src/bin/<name>.rs`. |

### 1.4 Empirical Findings — What We Verified Against kquery

Every load-bearing factual claim in this document about how kquery is *actually structured* (as opposed to claims about Rust idioms or workflow advice) was verified by running a command against the kquery source tree. The table below records each claim with the verifying command and the result. A reader can re-run any row to confirm the claim still holds, and any new claim added to this document later must come with a new row here.

Why this section exists: drafting a kquery-specific document from generic "Kotlin → Rust" priors produces wrong recommendations. Early drafts of this document had several such errors (claims that kquery used `suspend fun`, that the port needed `cargo-expand` for `data class` translation, that kquery used the Gang-of-Four visitor pattern); each survived several edit passes because nothing was grep-verified. The discipline going forward is that every kquery-shape claim earns its place by being executable.

| Claim | Verifying command | Result | Used in |
|---|---|---|---|
| kquery has zero `suspend fun` declarations | `grep -r "suspend fun" how-query-engines-work/ --include="*.kt"` | 0 matches | §3.9 |
| kquery uses `kotlinx.coroutines` in exactly three files, all for CPU parallelism via `runBlocking { async { } }` | `grep -rn "kotlinx.coroutines" how-query-engines-work/ --include="*.kt"` | 3 source files: `execution/ParallelContext.kt:135`, `examples/ParallelQuery.kt:29`, `benchmarks/Benchmarks.kt:89` | §3.9 |
| kquery does not use the Gang-of-Four visitor pattern | `grep -rn "Visitor\|accept(" how-query-engines-work/ --include="*.kt"` | 0 `Visitor` declarations, 0 `accept(...)` methods; one private helper function literally named `visit` in `SqlPlanner.kt:233` that is a plain recursive tree-walker, not a visitor protocol | §3.11 |
| kquery's dispatch idiom is Kotlin's `when` expression on sealed classes | `grep -rcE "^\s*when\s*[({]" how-query-engines-work/ --include="*.kt"` | 130+ usages across 54 files | §3.11, §3.14 |
| kquery's Flight server uses Java gRPC's synchronous blocking API | Inspection of `flight-server/src/main/kotlin/io/andygrove/kquery/flightserver/FlightServer.kt` and `KQueryFlightProducer.kt` — no `StreamObserver`, no `CompletableFuture`, no async gRPC stubs | confirmed synchronous | §3.9, §4.13 |
| The `physical-plan` module is the largest in the workspace (28 files) | `ls how-query-engines-work/physical-plan/src/main/kotlin/ how-query-engines-work/physical-plan/src/main/kotlin/expressions/ \| wc -l` | 28 `.kt` files | §4.6 |
| kquery hand-rolls a CSV parser rather than delegating to a library | Inspection of `datasource/src/main/kotlin/CsvDataSource.kt` | tokenisation, type inference, and `RecordBatch` assembly are all in-file | §4.2 |
| The `flight-server` Gradle module depends on **eight** sibling modules (not two) | `cat how-query-engines-work/flight-server/build.gradle.kts` | `datatypes`, `datasource`, `logical-plan`, `physical-plan`, `query-planner`, `sql`, `protobuf`, `execution` plus external `org.apache.arrow:flight-core` and `arrow-vector` | §4.13 |
| Neither the `client` nor the `distributed` Kotlin module declares a Gradle dependency on `:flight-server`; both talk to Flight servers over the wire via `org.apache.arrow:flight-core` | `cat how-query-engines-work/client/build.gradle.kts how-query-engines-work/distributed/build.gradle.kts` | `client` deps: `datatypes`, `datasource`, `logical-plan`, `protobuf` + external `flight-core`. `distributed` deps: `datatypes`, `datasource`, `logical-plan`, `physical-plan`, `query-planner`, `optimizer`, `execution`, `sql`, `protobuf` + external `flight-core`, `arrow-memory`, `arrow-vector`. **Zero in-workspace consumers of `flight-server`.** | §4.14, §4.15 |
| kquery has exactly two project-name-prefixed identifiers (`KQuery*`); both rebrand to `RQuery*` in the Rust port | `grep -rn "KQuery\w\+" how-query-engines-work/ --include="*.kt"` | Two identifiers: `KQueryFlightProducer` (its own file at `flight-server/.../KQueryFlightProducer.kt`) and `KQueryFlightServer` (defined inside `flight-server/.../FlightServer.kt`, referenced as Gradle `application.mainClass`). All other Kotlin identifiers are domain-named and translate mechanically per §3.0 | §3.0, §4.13 |

**Rule for future amendments.** Any new sentence added to this document that asserts something about how kquery is structured must add a row to this table — the claim, the command that proves it, the result. If the claim cannot be reduced to an executable check, it does not belong in the document.

---

## 2. Pre-Requisites and Tooling Setup

### 2.1 Software You Need

**Required.** Without these, you cannot build, test, or compare against the upstream Kotlin engine:

| Tool | Why | Install (macOS) |
|---|---|---|
| **JDK 17** | Build and run upstream `kquery` to verify behaviour against the reference | Standard JDK 17 install (Homebrew: `brew install openjdk@17`) |
| **Rust 1.85+** | This workspace targets edition 2024 | `curl https://sh.rustup.rs -sSf \| sh` |
| **cargo-nextest** | Faster, more capable test runner than `cargo test` | `cargo install cargo-nextest --locked` |
| **bacon** | Background `cargo check` watcher; instant feedback during the port | `cargo install bacon --locked` |
| **IntelliJ IDEA, RustRover, or VS Code** | Side-by-side reading of Kotlin source and Rust port. Install the JetBrains *Rust* plugin if using IntelliJ, or the `rust-analyzer` extension in VS Code | — |

**Optional, install only if a specific case demands it:**

- `just` — task runner for the `justfile` template in §2.3
- `cargo-flamegraph` — only if profiling becomes interesting
- `cargo-expand` — only if you ever need to see what a `#[derive(...)]` macro generated; rare for this port

### 2.2 Quick Verification After Cloning

```bash
cd how-query-engines-work-rust
cargo check --workspace      # downloads arrow-rs, tonic, etc.; should succeed
cargo build --workspace      # all 15 crates compile with empty bodies
cargo nextest run            # harness wires up; no tests defined yet
cargo run --bin nyc_taxi     # panics with `not yet implemented` and a pointer to the matching Kotlin file
```

Per-file 1:1 stubs already exist. Every Kotlin source file (`.kt`) has a corresponding empty Rust stub file (`.rs`) in the same crate, named by the PascalCase → snake_case convention described in §3.0. So `kquery/physical-plan/.../HashAggregateExec.kt` already has a matching empty `physical-plan/src/hash_aggregate_exec.rs` waiting to be filled in. Each crate's `lib.rs` declares them all via `pub mod <name>;`. Total: 83 stub files across 13 library crates plus 7 binary entries.

### 2.3 A Sample `justfile` for the Workspace

```make
default:
    @just --list

build:
    cargo build --workspace

watch:
    bacon check --all-targets

test:
    cargo nextest run

run BIN:
    cargo run --bin {{BIN}}

check:
    cargo fmt --all -- --check
    cargo clippy --all-targets -- -D warnings
    cargo doc --no-deps --workspace

tpch QUERY DATA:
    cargo run --release --bin tpch_runner -- {{QUERY}} {{DATA}}
```

---

## 3. The Kotlin → Rust Idiom Cheatsheet

This section has two parts. §3.0 covers **file-level and identifier naming** — the mechanical rule that decides which Rust file each Kotlin file maps to. §3.1 through §3.13 cover **language-level idioms** — how each Kotlin construct (sealed class, data class, interface, …) becomes its Rust equivalent, with side-by-side code. §3.14 is the **one-page summary table** to glance at while porting.

### 3.0 File and Identifier Naming Convention

**Files: `PascalCase.kt` → `snake_case.rs`**, applied mechanically with one well-known exception (acronym preservation) and one rebrand rule for the project name:

| Kotlin file | Rust file | Rule |
|---|---|---|
| `Schema.kt` | `schema.rs` | trivial — single word, lowercased |
| `LogicalPlan.kt` | `logical_plan.rs` | underscore between lowercase-then-uppercase boundary |
| `HashAggregateExec.kt` | `hash_aggregate_exec.rs` | same rule applied multiple times |
| `NYCTaxi.kt` | `nyc_taxi.rs` | **acronym preserved as one word** — split occurs only at the *last* uppercase letter before a lowercase one |
| `KQueryFlightProducer.kt` | **`r_query_flight_producer.rs`** | **Rebrand, not mechanical conversion** — see project-name-prefix rule below |

**Project-name-prefix rule (overrides mechanical conversion).** Kotlin identifiers that embed the project name — the `KQuery*` prefix — are *rebranded* to `RQuery*` in the Rust port, both in identifiers and in file names. The "K" stands for Kotlin and the "R" stands for Rust; the prefix tracks the project's identity, not a domain term. This affects exactly two identifiers in the upstream kquery codebase (verified in §1.4): `KQueryFlightProducer` (own file → file rebrands and the type rebrands) and `KQueryFlightServer` (defined inside `FlightServer.kt` → only the type rebrands; the file name stays `flight_server.rs` because `FlightServer` does not embed the project name). For every other identifier (`HashAggregateExec`, `LogicalPlan`, `NYCTaxi`, etc.) the strict mechanical rule above applies unchanged.

**Identifiers inside the file:** Rust convention for types is unchanged — `pub struct HashAggregateExec`, `pub enum LogicalExpr`, etc. (PascalCase). For functions and modules: snake_case. The Kotlin `class HashAggregateExec` and the Rust `pub struct HashAggregateExec` look identical at the type level; only the *file name* and the *module name* differ. The one exception is the project-name-prefix rule above.

**Module declarations:** each crate's `lib.rs` declares the per-file modules via `pub mod <snake_case_name>;`. For example, `physical-plan/src/lib.rs` contains 28 such declarations.

### 3.1 Sealed Class → Enum

The single highest-leverage translation. Kotlin's sealed-class hierarchies, including `LogicalExpr`, `LogicalPlan`, `PhysicalPlan`, and `Expression`, all become Rust enums.

```kotlin
// Kotlin
sealed class LogicalExpr {
    data class Column(val name: String) : LogicalExpr()
    data class Literal(val value: Any) : LogicalExpr()
    data class BinaryExpr(val op: String, val left: LogicalExpr, val right: LogicalExpr) : LogicalExpr()
}
```

```rust
// Rust
#[derive(Debug, Clone, PartialEq)]
pub enum LogicalExpr {
    Column(String),
    Literal(ScalarValue),                                  // typed; not `Any` — see ScalarValue note below
    BinaryExpr {
        op: String,
        left:  Box<LogicalExpr>,
        right: Box<LogicalExpr>,
    },
}
```

Two important details:

- **Recursive variants need `Box`** (or `Rc` / `Arc`). Rust enums are stack-allocated by default and recursive types must indirect through a heap pointer.
- **`Any` becomes a typed `ScalarValue` enum.** Kotlin's `Any` (and Java's `Object`) carries runtime type information for free; in Rust you declare the variants explicitly. This is one of the few places the port adds structure that wasn't in the Kotlin original — and it's the right structure to add.

### 3.2 Data Class → Struct With Derives

`data class` in Kotlin auto-generates `equals`, `hashCode`, `toString`, `copy`, and component accessors. Rust achieves this with derive macros.

```kotlin
data class Schema(val fields: List<Field>)
data class Field(val name: String, val dataType: ArrowType)
```

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Schema {
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Field {
    pub name:      String,
    pub data_type: ArrowType,
}
```

Add `Hash` only when the type is used as a HashMap key. Add `Eq` only when the type can support full equality (no `f64` fields — those force `PartialEq` only). The Kotlin `copy()` method becomes Rust's struct-update syntax (`Schema { fields: new_fields, ..schema }`).

### 3.3 Interface → Trait

```kotlin
interface DataSource {
    fun schema(): Schema
    fun scan(projection: List<String>): Sequence<RecordBatch>
}
```

```rust
pub trait DataSource {
    fn schema(&self) -> Schema;
    fn scan(&self, projection: &[String]) -> Box<dyn Iterator<Item = RecordBatch>>;
}
```

- Trait methods take an explicit `&self` (Rust has no implicit `this`).
- `List<String>` → `&[String]` for parameters (borrowed, cheap). Use `Vec<String>` for owned values.
- `Sequence<T>` → `Iterator<Item = T>` (see §3.5).
- For trait objects, use `Box<dyn Trait>` or `Arc<dyn Trait>` depending on whether the value crosses thread boundaries.

### 3.4 Companion Object → Associated Function

```kotlin
class CsvDataSource(private val path: String) : DataSource {
    companion object {
        fun fromUri(uri: String): CsvDataSource = ...
    }
}
```

```rust
impl CsvDataSource {
    pub fn from_uri(uri: &str) -> Self { ... }
}
```

Call it as `CsvDataSource::from_uri("...")` — the same shape as `CsvDataSource.fromUri("...")` in Kotlin. Constants in companion objects become module-level `pub const`.

### 3.5 `Sequence<T>` → `Iterator<Item = T>`

Kotlin's `Sequence` is a lazy, single-pass stream. Rust's `Iterator` is the direct analog.

```kotlin
fun scan(): Sequence<RecordBatch> = sequence {
    while (hasNext()) yield(nextBatch())
}
```

```rust
struct CsvScanIter { /* state */ }
impl Iterator for CsvScanIter {
    type Item = RecordBatch;
    fn next(&mut self) -> Option<RecordBatch> {
        if !self.has_next() { return None; }
        Some(self.next_batch())
    }
}
```

Return `Box<dyn Iterator<Item = RecordBatch>>` from trait methods. A future Rustified rewrite will switch the return type to `RecordBatchStream` (arrow-rs's `futures::Stream<Item = Result<RecordBatch>>`) so that scans can be cancelled, throttled, and composed with the rest of the async pipeline — but that's a separate workspace, not this one.

### 3.6 Throw → `panic!()`

This port deliberately uses `panic!()` and `unwrap()` everywhere the Kotlin code throws. *Not* idiomatic production Rust — a deliberate simplification for the faithful port.

```kotlin
if (schema == null) throw IllegalStateException("schema must not be null")
```

```rust
let schema = schema.expect("schema must not be null");
```

`expect` is preferred over bare `unwrap()` because the message survives into the panic.

### 3.7 Nullability `?` → `Option<T>`

```kotlin
val length: Int? = user?.name?.length
val name = user?.name ?: "anonymous"
```

```rust
let length: Option<usize> = user.as_ref().and_then(|u| u.name.as_ref()).map(|n| n.len());

// .as_deref() turns Option<String> into Option<&str> so the str literal
// "anonymous" can serve as the fallback without an owned String.
let name = user.as_ref().and_then(|u| u.name.as_deref()).unwrap_or("anonymous");
```

### 3.8 Lambdas

```kotlin
val doubled = numbers.map { it * 2 }
```

```rust
let doubled: Vec<i32> = numbers.iter().map(|n| n * 2).collect();
```

### 3.9 Coroutines — The Smaller Issue Than Expected

> This entry is intentionally longer than the others. The Kotlin-to-Rust coroutine question is the single most-asked question about ports between these two languages, so it earns the extra space to explain *why* the answer for this specific codebase is "almost a non-issue."

A reasonable assumption is that the kquery port will have to wrestle with Kotlin's `suspend fun` ↔ Rust's `async fn` mismatch. The actual data shows otherwise (verified in §1.4):

- **Zero `suspend fun` declarations** (across 1,135+ functions).
- **Three uses of `kotlinx.coroutines`**, all in `runBlocking { async { } }` form, all for the same purpose: **CPU parallelism during scan execution**. The three call sites are `execution/ParallelContext.kt`, `examples/ParallelQuery.kt`, and `benchmarks/Benchmarks.kt`.
- **No async I/O whatsoever.** No `StreamObserver`, no `CompletableFuture`, no async gRPC stubs — the Flight server uses Java gRPC's synchronous blocking API.

**What this means for the port.** The Rust translation is essentially **strict 1:1 with one well-bounded substitution**:

```kotlin
// Kotlin — the only async-shaped pattern in the entire kquery codebase
runBlocking(Dispatchers.Default) {
    val deferreds = inputs.map { input -> async { input.execute() } }
    deferreds.awaitAll()
}
```

```rust
// Rust — same semantics ("spawn parallel CPU work and join"), idiomatic form
use rayon::prelude::*;
let results: Vec<RecordBatch> = inputs
    .par_iter()
    .map(|input| input.execute())
    .collect();
```

For the cases where you need explicit task control rather than data parallelism (e.g., `ParallelContext` distributes work-queues round-robin and spawns one task per queue), use `rayon::scope` instead:

```rust
use std::sync::Mutex;

let results = Mutex::new(Vec::with_capacity(queues.len()));
rayon::scope(|s| {
    for queue in &queues {
        s.spawn(|_| {
            let partial = execute_partial_aggregate(queue);
            results.lock().unwrap().push(partial);
        });
    }
});
let results = results.into_inner().unwrap();
```

The substitution is *semantically* 1:1 — both versions spawn CPU-bound work onto a worker pool and join. The implementation primitive differs (Kotlin uses coroutines on `Dispatchers.Default`, Rust uses Rayon's work-stealing thread pool) but the *what* and *why* are identical. Add `rayon = "1"` as a dependency in the affected crates (`execution`, `examples`, `benchmarks`) and document the substitution in `TRANSLATION_NOTES.md`.

**The one place Rust async genuinely leaks in.** The Kotlin Flight server uses synchronous Java gRPC. The Rust equivalent — `tonic` — is async-only. So `flight-server/src/bin/flight_server.rs` and the `FlightService` impl must use `tokio` + `async fn`; everything inside the server delegates to `tokio::task::spawn_blocking` to call into the synchronous query engine. This is a *Rust-ecosystem-forced* divergence, not a translation issue, and it is the only one. Document it in `TRANSLATION_NOTES.md` as such.

### 3.10 Extension Functions → Trait + impl

Kotlin extension functions are typically translated as methods on a sealed `enum` (which gives you the same effect as a trait) or as a small trait that you implement for the target type:

```kotlin
fun LogicalPlan.print(): String = when (this) {
    is Projection -> "Projection: ${...}"
    is Scan -> "Scan: ${...}"
}
```

```rust
impl LogicalPlan {
    pub fn print(&self) -> String {
        match self {
            LogicalPlan::Projection(p) => format!("Projection: {}", p.format()),
            LogicalPlan::Scan(s)       => format!("Scan: {}", s.format()),
        }
    }
}
```

### 3.11 The Visitor Pattern Doesn't Apply (kquery uses `when`)

A reader coming from a typical "Kotlin → Rust" guide would expect a section here on translating the Gang-of-Four visitor pattern into `match`. **kquery doesn't use the visitor pattern, so there is nothing to translate.** Grep confirms it (see §1.4): zero `Visitor` declarations, zero `accept(...)` methods, zero double-dispatch protocols. Kotlin's own `when` expression on sealed classes is already the idiomatic equivalent of Rust's `match`, and Andy Grove uses `when` throughout — 130+ times across 54 files. Every one of those translates mechanically to a Rust `match` per §3.1 (sealed class → enum).

The one helper function in kquery literally named `visit` — `SqlPlanner.kt:233`, used to walk a `LogicalExpr` tree and collect column references — is a plain private recursive function with a `when (expr) { is Alias -> ..., is BinaryExpr -> ... }` body. No visitor *protocol*. Port it as a plain Rust function with a `match` body:

```rust
fn visit(expr: &LogicalExpr, accumulator: &mut HashSet<String>) {
    match expr {
        LogicalExpr::Column(name)     => { accumulator.insert(name.clone()); }
        LogicalExpr::Alias(a)         => visit(&a.expr, accumulator),
        LogicalExpr::BinaryExpr(b)    => { visit(&b.l, accumulator); visit(&b.r, accumulator); }
        LogicalExpr::AggregateExpr(a) => visit(&a.expr, accumulator),
        _ => {}
    }
}
```

Nothing about this is a "deviation from the Kotlin source" — both versions are a recursive function with an exhaustive case-analysis body. `when` and `match` are the same shape with different keywords.

### 3.12 Object Identity → Not a Rust Concept

Kotlin (and Java) has object identity — two distinct object references are not equal even if their fields are. Rust has no such concept; values are compared by `PartialEq`, and "sharing" is explicit via `Rc<T>` / `Arc<T>`. When you encounter Kotlin code that relies on identity (`===` checks), step back: identity is rarely semantically meaningful and the translation usually clarifies the intent.

### 3.13 Open Class → Not Needed

Rust types are sealed by default — there is no `open` keyword to add. The Kotlin pattern of `open class Foo` is replaced by either a `trait` (if subclasses extend behaviour) or an `enum` (if the variants are a known closed set). The `kquery` codebase uses `open` sparingly.

### 3.14 Cheatsheet Summary

The one-page recap. Bookmark this; the rest of §3 expands each row.

| Kotlin | Rust | Notes |
|---|---|---|
| `sealed class X { class A; class B }` | `enum X { A, B }` | Box recursive variants |
| `data class P(val x: T)` | `#[derive(...)] struct P { pub x: T }` | Derives: Debug, Clone, PartialEq, +Eq+Hash where appropriate |
| `interface I { fun f() }` | `trait I { fn f(&self); }` | `&self` is explicit |
| `companion object { fun g() }` | `impl X { pub fn g() }` | Call site: `X::g()` |
| `Sequence<T>` | `Iterator<Item = T>` | Synchronous |
| `throw E()` | `panic!(...)` or `.expect(...)` | Faithful-port simplification |
| `T?` | `Option<T>` | `?.` → `.and_then`; `?:` → `.unwrap_or` |
| `{ x -> x * 2 }` | `\|x\| x * 2` | Closures are zero-cost when monomorphised |
| `suspend fun` | N/A — kquery has zero `suspend fun` declarations | See §3.9 |
| `runBlocking { async { … } }` (CPU parallelism) | `rayon::par_iter().map(...).collect()` or `rayon::scope` | The only coroutine pattern in kquery; see §3.9 |
| `fun T.ext()` | `impl T { fn ext(&self) }` | Or trait + impl if shared across types |
| `when (x) { is A -> …; is B -> … }` | `match x { A(_) => …, B(_) => … }` | The default dispatch idiom in kquery — translates 1:1; see §3.1 |
| `===` (identity) | (usually no analogue) | Inspect the semantic intent |
| `open class` | `trait` or `enum` | Rust is sealed by default |
| `Any` | typed `enum` like `ScalarValue` | Lose the runtime cast; gain compile-time safety |
| `List<T>` | `Vec<T>` (owned) or `&[T]` (borrowed) | Params take borrowed; returns own |
| `Map<K, V>` | `HashMap<K, V>` | Default; use `BTreeMap` if ordering matters |
| `Pair<A, B>` | `(A, B)` tuple | First-class in Rust |
| `Unit` | `()` | Identical concept |
| `Nothing` | `!` (never type, nightly stable) | Rarely used in `kquery` |

---

## 4. The Module-by-Module Porting Plan

The 15 crates in build order. Build bottom-up: never start a module until everything in its "Depends on" column is done.

### 4.0 Reading the Module Entries

Every entry below follows the same six-field shape:

- **Kotlin source** — the upstream `.kt` files in `how-query-engines-work/<module>/src/main/kotlin/`.
- **Rust scaffold** — the matching crate directory; every Kotlin file already has an empty `.rs` stub at the matching snake_case path (per §2.2 and §3.0).
- **Key types** — the structs / enums / traits that anchor the module.
- **Dependencies** — internal crates (must be built first) and external dependencies.
- **Gotchas** — module-specific traps that have nothing to do with the §3 idiom translations.
- **Definition of done** — the executable check that proves the module is finished.

### 4.1 `datatypes` (Module 1 of 15)

- **Kotlin source:** 9 files: `ArrowFieldVector.kt`, `ArrowTypes.kt`, `ArrowVectorBuilder.kt`, `ColumnVector.kt`, `LiteralValueVector.kt`, `RecordBatch.kt`, `Schema.kt`, `Field.kt`, `ArrowAllocator.kt`.
- **Key types:** `Schema`, `Field`, `ArrowType` (enum), `ColumnVector` (trait), `ArrowFieldVector`, `LiteralValueVector`, `RecordBatch`.
- **Dependencies:** None internal. External: `arrow`, `arrow-array`, `arrow-schema`.
- **Gotchas:**
  - The Kotlin code wraps Java Arrow's `FieldVector`. The Rust port wraps `arrow_array::ArrayRef` (which is `Arc<dyn arrow_array::Array>`).
  - `RecordBatch` in arrow-rs is *already* a complete implementation. Re-export `arrow::record_batch::RecordBatch` and add Kotlin-style helpers as needed.
  - `ColumnVector` is the most-translated trait in the codebase — every operator dispatches through it. Get the API right before moving on.
- **Definition of done:** all Kotlin files have an equivalent in Rust; `cargo test -p datatypes` passes ports of the Kotlin unit tests; the public API can construct a `RecordBatch` with two `Int64` columns and iterate them.

### 4.2 `datasource` (Module 2 of 15)

- **Kotlin source:** 4 files: `DataSource.kt`, `CsvDataSource.kt`, `InMemoryDataSource.kt`, `ParquetDataSource.kt`.
- **Key types:** `DataSource` (trait), `CsvDataSource`, `InMemoryDataSource`, `ParquetDataSource`.
- **Dependencies:** `datatypes`. External: `arrow`, `parquet`.
- **Gotchas:**
  - CSV reader: use `arrow::csv::ReaderBuilder` instead of porting the hand-rolled Kotlin parser. Document the substitution in `TRANSLATION_NOTES.md`.
  - Parquet reader: use `parquet::arrow::ArrowReaderBuilder` (sync).
  - `InMemoryDataSource` is the simplest of the three — port it first to bootstrap the unit tests.
- **Definition of done:** can read a small CSV and a small Parquet file from `testdata/` and yield a stream of `RecordBatch`es.

### 4.3 `logical-plan` (Module 3 of 15)

- **Kotlin source:** 10 files: `LogicalPlan.kt`, `LogicalExpr.kt`, `DataFrame.kt`, `Scan.kt`, `Projection.kt`, `Selection.kt`, `Aggregate.kt`, `Join.kt`, `Limit.kt`, `Expressions.kt`.
- **Key types:** `LogicalPlan` (enum with variants `Scan`, `Projection`, `Selection`, `Aggregate`, `Join`, `Limit`), `LogicalExpr` (enum), `DataFrame` (builder).
- **Dependencies:** `datatypes`. External: `arrow`.
- **Gotchas:**
  - Recursive `LogicalPlan` variants must use `Box<LogicalPlan>` for the `input` field (see §3.1).
  - The Kotlin `DataFrame` is a builder that wraps a mutable `LogicalPlan`. In Rust, prefer a fluent builder that consumes `self` and returns `Self` — idiomatic Rust builder pattern.
  - The `schema()` method on every plan node must compute lazily *or* the schema must be cached. Compute eagerly on construction for this faithful port; cache later in a Rustified rewrite.
- **Definition of done:** can build a logical plan for `SELECT a, b FROM t WHERE a > 10 LIMIT 5` via the `DataFrame` builder; `plan.schema()` returns the correct projected schema.

### 4.4 `sql` (Module 4 of 15)

- **Kotlin source:** 7 files: `SqlTokenizer.kt`, `PrattParser.kt`, `SqlParser.kt`, `SqlPlanner.kt`, `Expressions.kt`, etc.
- **Key types:** `Token` (enum), `Tokenizer`, `PrattParser`, `SqlParser`, `SqlPlanner` (translates AST to `LogicalPlan`).
- **Dependencies:** `datatypes`, `logical-plan`.
- **⚠ Hard rule:** **do NOT swap in `sqlparser-rs`.** The Pratt parser is the pedagogical core of this module. Hand-port it from `PrattParser.kt`. The `sqlparser-rs` swap is a future-rewrite decision, not for this faithful port.
- **Gotchas:**
  - **Pratt parsing in one sentence:** each token type declares a numeric precedence and a parse function; the recursive `parse_expr(min_prec)` loop consumes tokens whose precedence ≥ `min_prec` and recurses with the operator's precedence + 1 for right-associativity. In Kotlin these precedence tables are expressed as class hierarchies; in Rust they map naturally to a `match` on token type that returns a precedence integer. If Pratt parsing is new, read the chapter in Andy's book before opening the source.
  - The Kotlin tokeniser uses regex; the Rust port can use the same approach with the `regex` crate, or do hand-rolled character matching. Hand-rolled is more aligned with the *faithful translation* discipline.
- **Definition of done:** parses and plans `SELECT a, b FROM t WHERE a > 10 ORDER BY b LIMIT 5` into a `LogicalPlan` matching the Kotlin reference (snapshot-tested with `insta`).

### 4.5 `optimizer` (Module 5 of 15)

- **Kotlin source:** 2 files: `Optimizer.kt`, `ProjectionPushDownRule.kt`.
- **Key types:** `OptimizerRule` (trait), `Optimizer` (orchestrates), `ProjectionPushDownRule`.
- **Dependencies:** `logical-plan`.
- **Gotchas:** smallest module — should take a few hours. Each rule is a stateless struct. Apply in a fixed order. Cost-based ordering is explicitly out of scope.
- **Definition of done:** `Optimizer::optimize(plan)` runs all registered rules and the output for the canonical test query matches the Kotlin reference.

### 4.6 `physical-plan` (Module 6 of 15)

- **Kotlin source:** 28 files (the largest module).
  - **Plan node:** `PhysicalPlan.kt`
  - **Operators:** `ScanExec.kt`, `ProjectionExec.kt`, `SelectionExec.kt`, `HashAggregateExec.kt`, `HashJoinExec.kt`, `LimitExec.kt`, `ShuffleReaderExec.kt`, `ShuffleWriterExec.kt`
  - **Expressions:** `AggregateExpression.kt`, `AvgExpression.kt`, `BinaryExpression.kt`, `BooleanExpression.kt`, `CastExpression.kt`, `ColumnExpression.kt`, `CountExpression.kt`, `DateExpression.kt`, `MathExpression.kt`, `MaxExpression.kt`, `MinExpression.kt`, `SumExpression.kt`, `UnaryMathExpression.kt`, `Expressions.kt`
  - **Shuffle / task:** `AggregateMode.kt`, `Action.kt`, `Task.kt`, `ShuffleLocation.kt`, `ShuffleManager.kt`
- **Key types:** `PhysicalPlan` (trait OR enum — see Gotchas), `Expression` (trait OR enum), all `*Exec` operators, all `*Expression` types, `AggregateMode`, `Action`, `Task`, `ShuffleLocation`, `ShuffleManager`.
- **Dependencies:** `datatypes`, `datasource`, `logical-plan`. External: `arrow`.
- **Gotchas:**
  - **Trait vs enum decision:** `PhysicalPlan` and `Expression` are open sets in spirit (more operators can be added) but a closed set in this codebase. The faithful translation uses `trait` + `Box<dyn Trait>` to match the Kotlin shape, even though enums would be slightly more idiomatic in Rust. Document the deviation possibility in `TRANSLATION_NOTES.md`.
  - **Hash aggregate is the trickiest operator.** It maintains per-key aggregator state; port carefully and snapshot-test against the Kotlin output for at least three input distributions.
  - **Hash join uses the same hash map machinery.** Port `HashAggregateExec` first, generalise the hash-table helper, then port `HashJoinExec`.
  - **Shuffle operators (`ShuffleReaderExec`, `ShuffleWriterExec`) depend on serialised plans** — they need `protobuf` to be functional. Build them with `unimplemented!()` bodies; finish in module 13 once `protobuf` is up.
  - **Honour the upstream file split.** The 28-file count is not arbitrary — Andy split this module finely so each file holds one cohesive concept (one operator, one expression).
- **Definition of done:** can execute `ScanExec → ProjectionExec → SelectionExec → LimitExec` end-to-end against a CSV scan and produce correct `RecordBatch`es. Hash aggregate and hash join work for canonical TPC-H test inputs. Shuffle operators are stubbed.

### 4.7 `query-planner` (Module 7 of 15)

- **Kotlin source:** `QueryPlanner.kt` (1 file).
- **Key types:** `QueryPlanner` (single `create_physical_plan(&LogicalPlan) -> Box<dyn PhysicalPlan>` method).
- **Dependencies:** `logical-plan`, `physical-plan`.
- **Gotchas:** smallest module. The translation is a single `match` on `LogicalPlan` variants that builds the matching `*Exec`.
- **Definition of done:** every `LogicalPlan` variant has a corresponding physical translation; the planner is invoked from `ExecutionContext` in module 8.

### 4.8 `execution` (Module 8 of 15) — Mid-Milestone

- **Kotlin source:** 2 files: `ExecutionContext.kt`, `ParallelContext.kt`.
- **Key types:** `ExecutionContext`, `ParallelContext` (uses Rayon for parallel scan execution).
- **Dependencies:** `logical-plan`, `physical-plan`, `query-planner`, `optimizer`, `datasource`. External: `arrow`, optionally `rayon` for parallel context.
- **Gotchas:**
  - This is the integration point. If everything above is right, this module is straightforward; if not, the bugs surface here.
  - `ParallelContext` uses Kotlin coroutines to run scans in parallel; use `rayon::scope` for a synchronous parallel fan-out per §3.9.
- **Definition of done:** `ExecutionContext::sql("SELECT ... FROM ...")` returns a `Vec<RecordBatch>` matching the Kotlin reference. **At the end of module 8 you have a working in-process query engine that can execute a small TPC-H query end-to-end.** Run `cargo run --release --bin tpch_runner -- q01.sql tpch_data/` and verify.

### 4.9 `fuzzer` (Module 9 of 15)

- **Kotlin source:** `Fuzzer.kt` (1 file).
- **Key types:** `Fuzzer` (random query generator). Uses the `rand` crate.
- **Dependencies:** `logical-plan`, `sql`.
- **Gotchas:** optional for the workspace definition of done. Useful for differential testing if you run upstream `kquery` in parallel and compare outputs.

### 4.10 `examples` (Module 10 of 15)

- **Kotlin source:** 4 files: `NYCTaxi.kt`, `ParallelQuery.kt`, `ParallelExecutionExample.kt`, `DistributedExample.kt`.
- **Key types:** four `main()` functions, one per binary.
- **Dependencies:** `execution`.
- **Gotchas:** `DistributedExample` requires `distributed` (module 15); leave it stubbed until then.
- **Definition of done:** `cargo run --release --bin nyc_taxi` produces output matching the Kotlin reference within rounding tolerance.

### 4.11 `benchmarks` (Module 11 of 15)

- **Kotlin source:** 2 files: `Benchmarks.kt`, `TpchRunner.kt`.
- **Key types:** two `main()` functions.
- **Dependencies:** `execution`.
- **Gotchas:** the TPC-H data (~1 GB) is not in `testdata/`; document the download step. Use the [DuckDB-generated TPC-H Parquet files](https://github.com/duckdb/duckdb/tree/main/test/data/tpch) for test data, or generate with `dbgen` from the upstream TPC-H tools.
- **Definition of done:** `cargo run --release --bin tpch_runner -- q01.sql tpch_data/` produces TPC-H Q1 results matching the Kotlin reference.

### 4.12 `protobuf` (Module 12 of 15)

- **Kotlin source:** 4 files: `PhysicalPlanSerializer.kt`, `PhysicalPlanDeserializer.kt`, `ProtobufSerializer.kt`, `ProtobufDeserializer.kt`. Plus `.proto` files under `protobuf/src/main/proto/`.
- **Key types:** generated code from `.proto` (in `OUT_DIR`); `PhysicalPlanSerializer`, `PhysicalPlanDeserializer`, `ProtobufSerializer`, `ProtobufDeserializer`.
- **Dependencies:** `physical-plan`. External: `prost`, `tonic-build`.
- **Gotchas:**
  - Copy `.proto` files verbatim from upstream to `proto/`.
  - Uncomment the relevant lines in `build.rs` once `.proto` files are in place.
- **Definition of done:** round-trip serialisation passes for the canonical physical plan.

### 4.13 `flight-server` (Module 13 of 15)

- **Kotlin source:** 2 files: `FlightServer.kt`, `KQueryFlightProducer.kt`. The binary `main()` lives inline in `FlightServer.kt` on the Kotlin side; in Rust this splits into a separate `src/bin/flight_server.rs` runnable that delegates into the library module.
- **Rust scaffold:** library modules at `src/flight_server.rs` and `src/r_query_flight_producer.rs`; binary entry at `src/bin/flight_server.rs` (registered explicitly via `[[bin]]` in `Cargo.toml`).
- **Key types:** `RQueryFlightServer` (`tonic::transport::Server`-based wrapper, lives in `src/flight_server.rs`; rebranded from Kotlin's `KQueryFlightServer` per the §3.0 project-name-prefix rule), `RQueryFlightProducer` (implements `arrow_flight::flight_service_server::FlightService`, lives in `src/r_query_flight_producer.rs`; rebranded from Kotlin's `KQueryFlightProducer`).
- **Dependencies (internal, per Kotlin Gradle):** `datatypes`, `datasource`, `logical-plan`, `physical-plan`, `query-planner`, `sql`, `protobuf`, `execution` — eight siblings. External: `arrow`, `arrow-flight`, `tonic`. See §1.4 for the verifying grep.
- **Reverse dependencies:** none in-workspace. Neither `client` nor `distributed` imports this crate — both talk to a running Flight server *over the network* via the external `arrow-flight` client.
- **Gotchas:**
  - `arrow-flight` requires an async runtime. Use a minimal `tokio` runtime *just for the server* — this is the one place where async leaks in (see §3.9). Wrap synchronous execution in `tokio::task::spawn_blocking`.
  - Listen on `0.0.0.0:50051` to match the upstream Kotlin server.
- **Definition of done:** the server starts; an external client (e.g., the Python `pyarrow.flight` client, or this workspace's own `client` crate once ported) can connect, submit a serialised logical plan, and receive Arrow RecordBatches back.

### 4.14 `client` (Module 14 of 15)

- **Kotlin source:** 2 files: `Client.kt`, `Context.kt`. The Kotlin `Client` wraps the external `org.apache.arrow.flight.FlightClient` directly — it does *not* import anything from the kquery `flight-server` module.
- **Key types:** `Client`, `Context`, `Endpoint`.
- **Dependencies (internal, per Kotlin Gradle):** `datatypes`, `datasource`, `logical-plan`, `protobuf`. External: `arrow`, `arrow-flight`, `tonic`. **No dependency on the `flight-server` crate** — the client speaks to a running Flight server over gRPC.
- **Gotchas:** mirrors the server's async story; use `tokio::runtime::Runtime` to call the async `arrow_flight::FlightServiceClient` from synchronous Rust.
- **Definition of done:** `Client::execute(plan)` round-trips through a local Flight server and returns the same `Vec<RecordBatch>` as in-process execution.

### 4.15 `distributed` (Module 15 of 15)

- **Kotlin source:** 5 files: `DistributedContext.kt`, `DistributedConfig.kt`, `DistributedPlanner.kt`, `QueryStage.kt`, `Scheduler.kt`. The Kotlin `Scheduler` defines its own `ExecutorClient` interface internally; the *concrete implementation* is expected to be a separate piece of glue.
- **Key types:** `DistributedContext`, `DistributedConfig`, `DistributedPlanner`, `QueryStage`, `Scheduler`, `ExecutorClient` (trait).
- **Dependencies (internal, per Kotlin Gradle):** `datatypes`, `datasource`, `logical-plan`, `physical-plan`, `query-planner`, `optimizer`, `execution`, `sql`, `protobuf` — nine siblings. External: `arrow`, `arrow-flight`. **No dependency on `flight-server` or `client`.**
- **Gotchas:**
  - The most complex module after `physical-plan`. The Kotlin original is the minimal-viable Ballista equivalent and even that is non-trivial.
  - The scheduler can be naive — round-robin or random assignment. Sophisticated scheduling is out of scope.
- **Definition of done:** `DistributedExample` runs a query split across two locally-spawned Flight workers (each running the `flight-server` binary as a separate process) and produces the correct result.

---

## 5. Testing Strategy

The kquery test suite is the most authoritative spec for what each piece of the engine does. Port the Kotlin tests one-for-one alongside the production code, plus two layers above that — snapshot tests for the planner trees and end-to-end smoke tests for the full pipeline.

### 5.1 Three Test Layers

| Layer | Tool | What it tests |
|---|---|---|
| **Unit** | `#[test]` per crate | Internal logic; ports of Kotlin `JUnit` tests one-for-one |
| **Plan-tree snapshots** | [`insta`](https://docs.rs/insta) crate | Logical and physical plan structures — large but stable format |
| **End-to-end** | `#[test]` in `examples/tests/` and the example binaries | NYC taxi sums, TPC-H Q1 result rows |

### 5.2 Unit Test Conventions

- Use `pretty_assertions::assert_eq!` instead of the standard `assert_eq!` — the diff output is dramatically better for struct comparisons.
- Run with `cargo nextest run` — faster than `cargo test` and produces cleaner output.
- For each Kotlin test file (`<Module>Test.kt`), create a corresponding `tests/<module>_test.rs` integration test or inline `mod tests { ... }` in `lib.rs`.

### 5.3 Snapshot Testing With `insta`

```rust
#[test]
fn projection_pushdown_through_filter() {
    let plan = parse_and_plan("SELECT a FROM t WHERE a > 10 AND b < 5");
    let optimized = Optimizer::new().optimize(&plan);
    insta::assert_debug_snapshot!(optimized);
}
```

First run captures a `.snap` file; subsequent runs assert against it. `cargo insta review` reviews and accepts diffs interactively.

### 5.4 End-to-End Smoke Tests

```bash
cargo run --release --bin nyc_taxi
cargo run --release --bin tpch_runner -- queries/q01.sql tpch_data_sf1/
```

If either fails after a "successful" module is declared done, the module's test coverage was insufficient.

### 5.5 Differential Testing Against Upstream `kquery`

Once `fuzzer` is functional (module 9), it can generate random valid queries and run them through both engines and compare. Optional but high-leverage.

---

## 6. Development Workflow

### 6.1 The Per-Module Checklist

A repeatable per-module workflow:

1. **Read the Kotlin source completely** before writing any Rust — even small files.
2. **Read the corresponding chapter** in *How Query Engines Work*. Andy's prose explains *why* each design choice was made.
3. **Open the matching Rust stubs.** Per the file-level 1:1 scaffolding (§2.2 and §3.0), every Kotlin `.kt` file already has an empty Rust `.rs` stub at the matching path.
4. **Read the Kotlin tests for the module.** They are the most authoritative spec.
5. **Port the data types first** — structs, enums, traits. Don't write method bodies yet; let them compile with `todo!()`.
6. **Port one method at a time, with its test.** Red-green workflow.
7. **Run `cargo fmt` and `cargo clippy` before every commit.** Never carry warnings forward.
8. **Commit per logical unit** (a type + its tests, or a method + its tests).
9. **Update `TRANSLATION_NOTES.md`** the *same commit* as any deliberate divergence — the arrow-rs CSV substitution, the Rayon-for-coroutines swap, the tonic-forced async boundary in the Flight server. Batching divergences for a "cleanup commit later" never works.
10. **Run the full-workspace smoke test** (`cargo nextest run --workspace`) before moving to the next module.

### 6.2 The `TRANSLATION_NOTES.md` File

See [`TRANSLATION_NOTES.md`](TRANSLATION_NOTES.md) at the workspace root. The file documents every deliberate divergence from the Kotlin source. Format: one entry per divergence, grouped by module, with date / subject / substitution / rationale / cross-reference.

### 6.3 Commit Conventions

Conventional Commits:

```
feat(datatypes):   port ColumnVector trait and ArrowFieldVector impl
feat(sql):         port Pratt parser precedence table
test(optimizer):   port ProjectionPushDownRuleTest
fix(physical):     correct HashAggregate state merge for partial mode
docs(workspace):   add TPC-H test data setup instructions
chore(deps):       bump arrow-rs to 55.1
```

### 6.4 When to Read the Book vs the Source

- **Read the book chapter** when starting a new module. Andy's prose explains *why*.
- **Read the Kotlin source** when writing each method. The source is the spec.
- **Read the Kotlin tests** when verifying each method. The tests are the acceptance criteria.

---

## 7. Common Pitfalls

### 7.1 Over-Engineering Early Modules

You will be a better Rust engineer at module 6 than you were at module 1. The temptation to rewrite Module 1 with the techniques you learned in Module 6 is real and *should be resisted*. Refactors belong in a future Rustified rewrite, not in this faithful port.

### 7.2 Borrow-Checker Fights That Suggest Redesign

When the borrow checker complains, the answer is almost always `.clone()`. Use `#[derive(Clone)]` aggressively. Resist the urge to redesign for borrow correctness; that belongs in a future rewrite.

### 7.3 Async Creep

Tokio and `async fn` are tempting once you reach the Flight modules. **Don't.** The kquery codebase has *zero* `suspend fun` declarations; it is already synchronous from end to end. The only three uses of `kotlinx.coroutines` are CPU parallelism that translates to Rayon, not Tokio (see §3.9). The one place async genuinely leaks in is the Flight server, where `tonic` is async-only — and even there, wrap synchronous logic in `tokio::task::spawn_blocking` rather than going async throughout.

### 7.4 Over-Importing arrow-rs

This faithful port should use arrow-rs sparingly — primarily for `RecordBatch`, `Schema`, `Field`, `ArrayRef`, and the CSV/Parquet readers. Don't reach for `arrow::compute::*` for filter/projection logic; the Kotlin version implements these itself, and so should the Rust port. The exception is when arrow-rs *already provides* something the Kotlin code laboriously reimplements (e.g., CSV parsing) — document the substitution in `TRANSLATION_NOTES.md`.

### 7.5 Premature Use of `sqlparser-rs`

Do NOT swap in `sqlparser-rs` for the Pratt parser. The hand-port is the pedagogical core of the SQL module. The `sql/Cargo.toml` already has `sqlparser` commented out as a hard reminder.

### 7.6 Importing Large Test Data into the Repo

NYC taxi (~3 GB) and TPC-H scale factor 1 (~1 GB) are *not* checked in. The `testdata/` directory holds only small synthetic fixtures. The `benchmarks/` README documents the download steps for the real data sets.

### 7.7 JVM Thinking Carrying Over

- **Object identity comparisons** (`===`) → meaningless in Rust; usually the code wanted structural equality (`==`), which becomes `PartialEq`.
- **Reflection** (`obj.javaClass`, `KClass<T>`) → not available; refactor the design.
- **Mutable shared state without explicit synchronisation** → won't compile; use `Arc<Mutex<T>>` or `RwLock`.
- **Try-with-resources / `use`** → use `Drop` impls (RAII).

### 7.8 Being Too Clever With Type-State and Lifetimes

This faithful port should *not* introduce type-state encodings, sophisticated lifetime parameters, or generic-ified what was concrete in Kotlin. These belong in a future Rustified rewrite.

### 7.9 Forgetting to Update `TRANSLATION_NOTES.md`

`TRANSLATION_NOTES.md` is the audit trail for everything that deliberately diverges. Update it the same commit as the divergence; do not batch up entries.

---

## 8. Definition of Done

### 8.1 Per-Module Definition of Done

Each module is "done" when:

- [ ] All Kotlin source files in the module have an equivalent in Rust.
- [ ] `cargo build -p <module>` succeeds with zero warnings.
- [ ] `cargo nextest run -p <module>` passes — including ports of all Kotlin unit tests.
- [ ] Public types have at least a one-line doc comment (rustdoc).
- [ ] `cargo doc --no-deps -p <module>` builds without warnings.
- [ ] `cargo clippy -p <module> --all-targets -- -D warnings` passes.
- [ ] Any deviations from the Kotlin source are recorded in `TRANSLATION_NOTES.md`.

### 8.2 Workspace-Level Definition of Done

The port as a whole is complete when:

- [ ] `cargo build --workspace` succeeds with zero warnings.
- [ ] `cargo nextest run` passes — all 15 crates.
- [ ] `cargo run --release --bin nyc_taxi` produces output matching the Kotlin reference within rounding tolerance.
- [ ] `cargo run --release --bin tpch_runner -- queries/q01.sql tpch_data/` produces TPC-H Q1 results matching the Kotlin reference.
- [ ] `DistributedExample` runs successfully across two locally-spawned Flight workers.
- [ ] `TRANSLATION_NOTES.md` is complete and accurate.
- [ ] The workspace README documents the current state, how to run the smoke tests, and the relationship to upstream `kquery`.

---

## Closing Note

This document optimises for one thing: making the port mechanical enough that progress is uninterrupted. The §3 cheatsheet decides every language-level question. The §4 module entries decide every file-level question. The §5–§7 sections handle the workflow around the work. None of it is original — it is a curation of what Andy Grove already designed and what the Rust community has already standardised on.

The methodological rule that earns this document its accuracy is in §1.4: **every claim about how kquery is actually structured comes from a grep, not from generic Kotlin-to-Rust priors.** Anyone amending the document later must add a row to §1.4's empirical-findings table for each new structural claim about kquery, with the command that verifies it.

When in doubt during the port, the discipline is always the same:

> *The whole point of a faithful port is that the original author's design choices teach you something every time you encounter one. Resist redesigning. A future Rustified rewrite is for the redesign; this port is for the apprenticeship.*
