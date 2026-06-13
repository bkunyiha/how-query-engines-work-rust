//! # physical-plan
//!
//! Physical execution plans, operators, and expression evaluation.
//!
//! ## Kotlin source
//! Faithful port of `kquery/physical-plan/src/main/kotlin/` (28 files):
//! - **Plan node:** `PhysicalPlan.kt`
//! - **Operators:** `ScanExec.kt`, `ProjectionExec.kt`, `SelectionExec.kt`,
//!   `HashAggregateExec.kt`, `HashJoinExec.kt`, `LimitExec.kt`,
//!   `ShuffleReaderExec.kt`, `ShuffleWriterExec.kt`
//! - **Expressions:** `AggregateExpression.kt`, `AvgExpression.kt`,
//!   `BinaryExpression.kt`, `BooleanExpression.kt`, `CastExpression.kt`,
//!   `ColumnExpression.kt`, `CountExpression.kt`, `DateExpression.kt`,
//!   `MathExpression.kt`, `MaxExpression.kt`, `MinExpression.kt`,
//!   `SumExpression.kt`, `UnaryMathExpression.kt`, `Expressions.kt`
//! - **Shuffle / task:** `AggregateMode.kt`, `Action.kt`, `Task.kt`,
//!   `ShuffleLocation.kt`, `ShuffleManager.kt`
//!
//! Largest module in the workspace. Keep each generated file under
//! 25 lines of executable code where possible — split at function boundaries.
//!
//! ## Status
//! Module 6 of 15 — all four phases ported (per ARCHITECTURE.md §4.6). The only
//! deferred pieces are the shuffle operators' context-driven execution and
//! `ShuffleManager`'s Arrow-IPC I/O, which belong to the distributed modules (13/14).
//! - **Phase 1 (done):** the `PhysicalPlan` and `Expression` traits, and the
//!   simple expressions — column, literals, binary (with numeric coercion),
//!   boolean comparisons/logical, math, and cast. `BooleanExpressionTest` and
//!   `CastExpressionTest` are ported.
//! - **Phase 2 (done):** the straightforward operators — `ScanExec`,
//!   `ProjectionExec`, `SelectionExec`, `LimitExec` — execute `ScanExec →
//!   Selection → Projection → LimitExec` end-to-end over a CSV scan. Building an
//!   output batch from evaluated columns goes through the new
//!   `datatypes::record_batch::create` bridge (the arrow `RecordBatch` holds
//!   `ArrayRef`s, so virtual literal columns are materialized).
//! - **Phase 3 (done):** the two remaining stateless scalar expressions —
//!   `DateExpression` (date ± interval) and `UnaryMathExpression` (`Sqrt`/`Log`) —
//!   plus the stateful aggregation core: `AggregateExpression`, the five
//!   aggregates (`Min`/`Max`/`Sum`/`Count`/`Avg`, each with an `Accumulator`
//!   impl), `AggregateMode`, and `HashAggregateExec` (group-by hash aggregation).
//!   `AccumulatorValue` carries AVG's compound (sum, count) intermediate state.
//!   `AggregateTest` (the three accumulator tests) is ported, plus a group-by
//!   integration test over `employee.csv`.
//! - **Phase 4 (done):** `HashJoinExec` (equi-join: build on the right, probe with
//!   the left; `Inner`/`Left`/`Right`), reusing the shared `row_key::RowKey` hash
//!   helper. The shuffle/task scaffolding types (`Action`/`QueryAction`/
//!   `ShuffleIdAction`, `Task`, `ShuffleLocation`) are ported as data types. The
//!   shuffle operators (`ShuffleReaderExec`, `ShuffleWriterExec`) and `ShuffleManager`
//!   I/O are stubbed with `unimplemented!()` — their `execute()` requires the
//!   distributed executor context (modules 13/15), exactly as Kotlin's `execute()`
//!   throws and defers to `executeWithContext`/`executeAndWriteShuffle`.
//!
//! The per-phase file inventory and what each phase does is tabulated in
//! ARCHITECTURE.md §4.6 ("Porting phases").
//!
//! ## Design — traits, not enums (the documented §4.6 deviation)
//! `PhysicalPlan` and `Expression` are Rust **traits** referenced through
//! `Arc<dyn PhysicalPlan>` / `Arc<dyn Expression>`, *not* enums. This reverses the
//! §3.1 "interface hierarchy → enum" rule applied to `logical_plan`, because the
//! physical operator/expression set is large (28 files) and open in spirit. See
//! the file-level docs and `TRANSLATION_NOTES.md` for the rationale.

// ==============================================================
// Per-file modules — one for each upstream Kotlin source file.
// ==============================================================
pub mod action;
pub mod aggregate_expression;
pub mod aggregate_mode;
pub mod avg_expression;
pub mod binary_expression;
pub mod boolean_expression;
pub mod cast_expression;
pub mod column_expression;
pub mod count_expression;
pub mod date_expression;
// `executor_context.rs` has no Kotlin counterpart in this directory — it
// bundles the four constructor params of Kotlin's `KQueryFlightProducer`
// (executorId/host/port + shuffleManager) into a single value, so the
// shuffle operators (Batches B/C) take one parameter instead of four.
// See `ARCHITECTURE.md` §1.5 for the Phase 2 plan to move this struct
// to `flight-server` alongside `ShuffleManager`.
pub mod executor_context;
pub mod expressions;
pub mod hash_aggregate_exec;
pub mod hash_join_exec;
pub mod limit_exec;
pub mod math_expression;
pub mod max_expression;
pub mod min_expression;
pub mod physical_plan;
pub mod projection_exec;
pub mod scan_exec;
pub mod selection_exec;
pub mod shuffle_location;
pub mod shuffle_manager;
pub mod shuffle_reader_exec;
pub mod shuffle_writer_exec;
pub mod sum_expression;
pub mod task;
pub mod unary_math_expression;

// Internal helper with no Kotlin counterpart: a float-aware hashable row key,
// used by `HashJoinExec` for its join keys (and the same shape `HashAggregateExec`
// uses for group keys). See `row_key.rs` and ARCHITECTURE.md §4.6.
mod row_key;

// ==============================================================
// Re-exports for convenient downstream `use physical_plan::*;` ergonomics.
// Only the phase-1 types are exported so far; later phases add to this list.
// ==============================================================
pub use binary_expression::BinaryExpression;
pub use boolean_expression::{
    AndExpression, BooleanExpression, EqExpression, GtEqExpression, GtExpression, LtEqExpression,
    LtExpression, NeqExpression, OrExpression,
};
pub use cast_expression::CastExpression;
pub use column_expression::ColumnExpression;
pub use expressions::{
    Accumulator, AccumulatorValue, Expression, LiteralDateExpression, LiteralDoubleExpression,
    LiteralIntervalDaysExpression, LiteralLongExpression, LiteralStringExpression,
};
pub use math_expression::{
    AddExpression, DivideExpression, MathExpression, MultiplyExpression, SubtractExpression,
};
pub use physical_plan::{format, PhysicalPlan};
// Phase-2 operators.
pub use limit_exec::LimitExec;
pub use projection_exec::ProjectionExec;
pub use scan_exec::ScanExec;
pub use selection_exec::SelectionExec;
// Phase-3 scalar expressions.
pub use date_expression::{DateAddIntervalExpression, DateSubtractIntervalExpression};
pub use unary_math_expression::{Log, Sqrt, UnaryMathExpression};
// Phase-3 aggregation.
pub use aggregate_expression::AggregateExpression;
pub use aggregate_mode::AggregateMode;
pub use avg_expression::{AvgAccumulator, AvgExpression};
pub use count_expression::{CountAccumulator, CountExpression};
pub use hash_aggregate_exec::HashAggregateExec;
pub use max_expression::{MaxAccumulator, MaxExpression};
pub use min_expression::{MinAccumulator, MinExpression};
pub use sum_expression::{SumAccumulator, SumExpression};
// Phase-4 join + shuffle/task scaffolding.
pub use action::{Action, QueryAction, ShuffleIdAction};
pub use hash_join_exec::HashJoinExec;
pub use shuffle_location::ShuffleLocation;
pub use shuffle_manager::ShuffleManager;
pub use shuffle_reader_exec::ShuffleReaderExec;
pub use shuffle_writer_exec::ShuffleWriterExec;
pub use task::Task;
// Module 13 scaffolding — consumed by flight-server (Batches D/E) and the
// shuffle operator bodies (Batches B/C).
pub use executor_context::ExecutorContext;
