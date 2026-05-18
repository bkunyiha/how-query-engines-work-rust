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
//! Largest module in the workspace. Plan §1.4: keep each generated file under
//! 25 lines of executable code where possible — split at function boundaries.
//!
//! ## Status
//! TODO — module 6 of 15.

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
