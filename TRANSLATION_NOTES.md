# Translation Notes — Kotlin `kquery` → Rust `rquery`

This file documents every place the Rust port deliberately diverges from the Kotlin source.

The [`ARCHITECTURE.md`](ARCHITECTURE.md) §3 cheatsheet covers the *standard* substitutions (`sealed class` → `enum`, `data class` → `struct`, `throw` → `panic!`, Kotlin `when` → Rust `match`, etc.) — those do not need entries here. This file captures the *non-standard* ones: deviations from the default translation strategy. Examples of what belongs here include library-forced substitutions (e.g., using arrow-rs's `csv::ReaderBuilder` instead of hand-rolling the CSV parser kquery implements in `CsvDataSource.kt`), ecosystem-forced substitutions (e.g., the Flight server going async because `tonic` requires it), CPU-parallelism substitutions (the three Kotlin `runBlocking { async { } }` sites becoming Rayon), and any other place where the Rust code does not mirror the Kotlin shape line-for-line.

Required by the port's [definition of done](README.md#definition-of-done).

## Convention

One entry per deliberate deviation. Each entry contains:

- **Subject** — the Kotlin construct, file, or behaviour that was not directly translated.
- **Substitution** — the Rust replacement, in one sentence.
- **Rationale** — the reason the standard translation didn't apply.
- **Cross-reference** — the relevant section of the Translation Plan, where applicable.

Entries are grouped by module (matching the §4 module-by-module porting plan). Within a module, entries are listed in the order they were added (the same order the porter encountered the divergences).

**Update this file in the same commit as the diverging code.** Batching divergences for a "cleanup commit later" never works — by the time you come back, the rationale is gone. This is the single rule that determines whether the file remains a useful audit trail or rots into stale prose. The commit history serves as the date record; the entries themselves don't need timestamps.

---

## Per-Module Log

### Module: datatypes

- **`ScalarValue` enum added (no Kotlin counterpart).** Kotlin
  `ColumnVector.getValue(i: Int): Any?` returns a nullable runtime-typed
  object using the JVM's `Any`. Rust has no equivalent type-erased exhaustive
  return, so the port introduces a typed `ScalarValue` enum in a new file
  `scalar_value.rs`. One variant per Arrow scalar type, plus a `Null` variant
  in place of `?` nullability. **Cross-reference:** ARCHITECTURE.md §3.1
  (the "`Any` becomes a typed `ScalarValue` enum" rule).
- **`ArrowAllocator` not ported.** The Kotlin
  `object ArrowAllocator { val rootAllocator = RootAllocator(Long.MAX_VALUE) }`
  wraps Java Arrow's `RootAllocator`. arrow-rs uses its own `Arc`-backed
  memory model (`Buffer`) with no separate allocator abstraction at this
  level. Vector construction in arrow-rs goes through typed builders
  (`Int32Builder::new()`, etc.) that allocate directly. The `ArrowAllocator`
  object simply has no equivalent — and isn't needed.
- **`FieldVectorFactory.create(...)` not ported.** The Kotlin
  factory function builds a typed `FieldVector` (BitVector, IntVector, etc.)
  ready for indexed mutation. arrow-rs has no `FieldVector`-equivalent open
  abstraction; the equivalent role is filled by per-type builders, handled
  in `arrow_vector_builder.rs`. Folded into the `ArrowVectorBuilder::new`
  constructor.
- **`AutoCloseable` / `ColumnVector.close()` not ported.**
  Kotlin's `interface ColumnVector : AutoCloseable` requires explicit close
  for memory release. arrow-rs's `ArrayRef` is `Arc<dyn Array>` — memory is
  released when the last reference is dropped. The Rust `ColumnVector` trait
  has no `close()` method; `Drop` handles any non-Arrow resources
  automatically.
- **`ArrowVectorBuilder` API: indexed-set → typed-append.**
  Kotlin's `ArrowVectorBuilder.set(i: Int, value: Any?)` writes at an index.
  arrow-rs builders are append-only (`Int32Builder::append_value(v)` +
  `append_null()`), and column construction is strictly row-order. The
  Rust port exposes `append_value(&ScalarValue)` / `append_null()` instead
  of indexed `set`. Every kquery call site uses sequential indices anyway,
  so the semantics survive. A no-op `set_value_count(_n)` shim is kept for
  Kotlin source-code shape compatibility. **Cross-reference:**
  ARCHITECTURE.md §3.5 (`Sequence<T>` → `Iterator<Item = T>` reasoning
  applies to the builder shape too).
- **`RecordBatch` re-exported from arrow-rs, not wrapped.**
  Per ARCHITECTURE.md §4.1's explicit rule, the Rust port re-exports
  `arrow_array::RecordBatch` rather than wrapping it. The Kotlin-style
  helpers (`row_count`, `column_count`, `field(i)`, `to_csv`) are provided
  as free functions in `record_batch.rs`.
- **`Decimal128Vector` / `Decimal256Vector` not yet ported.**
  Kotlin's `ArrowFieldVector` and `ArrowVectorBuilder` handle Decimal via
  Java's `BigDecimal`. The arrow-rs equivalents are `Decimal128Array` /
  `Decimal256Array` with precision + scale carried in the `DataType`.
  Deferred until a downstream module actually needs them; will add
  matching `ScalarValue::Decimal*` variants then.
- **Snake_case method renames.** `getType` / `getValue` →
  `get_type` / `get_value` per Rust convention. Idiom-level, not a substantive
  change, but listed here for completeness.
- **Observation (not a deviation): `ShuffleId` and `ShuffleLocation` in
  upstream `datatypes/` are unreachable code.** Grep across the entire
  upstream kquery Kotlin tree (`grep -rn "ShuffleId\b" --include="*.kt"`
  and the same for `ShuffleLocation`) shows:
  - `datatypes/ShuffleId.kt` is referenced only by `physical-plan/Action.kt`,
    which defines `data class ShuffleIdAction(val shuffleId: ShuffleId)` —
    but `ShuffleIdAction` is itself never constructed anywhere. The
    `ProtobufDeserializer.fromProto(action: Action)` `when` block only
    builds `QueryAction`; every other branch throws `NotImplementedError`.
    So the chain is `ShuffleId → ShuffleIdAction → (nothing)`, dead.
  - `datatypes/ShuffleLocation.kt` (4 fields: `jobUuid`, `stageId`,
    `partitionId`, `executionUuid`) is referenced by no consumer. The
    *live* `ShuffleLocation` is a separate, differently-shaped definition
    in `physical-plan/ShuffleLocation.kt` (6 fields: adds `executorId`,
    `executorHost`, `executorPort` for Flight RPC). All actual usages
    in `distributed/`, `flight-server/`, `examples/`, `protobuf/`, and
    `SchedulerTest.kt` import `io.andygrove.kquery.physical.ShuffleLocation`,
    not `io.andygrove.kquery.datatypes.ShuffleLocation`.

  Both Rust files (`shuffle_id.rs`, `shuffle_location.rs`) are kept and
  exported from `lib.rs` as-is, per the faithful-port discipline (dead
  code in upstream stays dead code in the port; pruning is a redesign
  decision for a future Rustified rewrite, not for this port). When
  module 6 (`physical-plan`) is ported, the live 6-field
  `physical/ShuffleLocation` will be added at `physical-plan/src/shuffle_location.rs`,
  giving the Rust port two `ShuffleLocation` types mirroring kquery exactly.
  `ShuffleIdAction` in `physical-plan/Action.kt` will likewise be ported
  as the dead-but-defined data class it is in kquery.

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

> *Every entry above is one place a future reviewer should be able to ask "why?" and get a precise answer pointing back to the planning rationale. If an entry can't survive that test, expand it or remove it.*
