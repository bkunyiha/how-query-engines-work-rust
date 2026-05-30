//! Port of `kquery/fuzzer/src/main/kotlin/Fuzzer.kt`.
//!
//! Random `LogicalPlan` / `RecordBatch` / `LogicalExpr` generator for differential
//! testing. A `Fuzzer` is seeded deterministically so the same Rust run reproduces
//! the same sequence of batches and plans.
//!
//! ## Translation notes
//! - **`kotlin.random.Random(0)` → `rand::rngs::StdRng::seed_from_u64(0)`** (workspace
//!   `rand` dep). The Rust RNG is deterministic *across Rust runs*; its byte sequence
//!   does **not** match Kotlin's (different algorithms). The fuzzer is used for
//!   differential testing inside Rust — not for cross-language reproducibility.
//! - **`Any?` → `ScalarValue`** (per `ARCHITECTURE.md` §3 cheatsheet). The Kotlin
//!   `createValues : List<Any?>` becomes `create_values -> Vec<ScalarValue>`, and the
//!   columns parameter of `createRecordBatch(schema, columns)` becomes
//!   `Vec<Vec<ScalarValue>>`.
//! - **Kotlin overloaded `createRecordBatch` resolved into two distinct names.**
//!   Rust can't overload by argument type, so the two Kotlin signatures map to:
//!   - `createRecordBatch(schema, n)` → [`Fuzzer::create_random_record_batch`].
//!   - `createRecordBatch(schema, columns)` → [`Fuzzer::create_record_batch`].
//!   The shorter name goes to the from-columns version because that is the variant
//!   the dependent tests (`BooleanExpressionTest`, `CastExpressionTest`, most of
//!   `ExecutionTest`) call most often.
//! - **`Fuzzer.rand` + `Fuzzer.enhancedRandom` (sharing one `Random`) → one field
//!   on `Fuzzer`.** Kotlin gives `Fuzzer` two fields that wrap the *same* RNG
//!   instance; reproducing that shared-mutable-state shape in Rust would force
//!   interior mutability for no gain. Instead, `Fuzzer` holds a single
//!   [`EnhancedRandom`] which owns the underlying `StdRng` and exposes both the
//!   biased helpers (`next_byte`, `next_double`, …) and a raw [`EnhancedRandom::rng`]
//!   accessor for the places Kotlin called `rand.nextInt(N)` directly.
//! - **`VectorSchemaRoot.allocateNew()` + per-type `set(row, value)` dispatch
//!   → [`datatypes::ArrowVectorBuilder::append_value`].** The Rust builder already
//!   performs the per-type variant dispatch (see `arrow_vector_builder.rs`), so the
//!   batch-construction loop is one line per column instead of an explicit
//!   `when (v) { is IntVector -> v.set(...); … }` block.
//! - **`self.rng.r#gen::<T>()` (escaped identifier).** `gen` is a reserved keyword
//!   in Rust 2024 (the workspace edition), so calls to `rand::Rng::gen` must use
//!   the raw-identifier escape `r#gen`. The method name itself is unchanged; only
//!   the syntax differs. (Rand 0.9 added a non-keyword alias `random()` for this
//!   reason; the workspace pins `rand = "0.8"`, hence the escape.)

use datatypes::{ArrowVectorBuilder, ColumnVector, RecordBatch, ScalarValue, Schema, record_batch};
use logical_plan::{DataFrame, LogicalExpr};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use arrow_schema::DataType;

/// Character pool for [`EnhancedRandom::next_string`] — `a-z`, `A-Z`, `0-9`.
/// Kotlin: `private val charPool: List<Char> = ('a'..'z') + ('A'..'Z') + ('0'..'9')`.
const CHAR_POOL: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

// ---------------------------------------------------------------------------
// Fuzzer
// ---------------------------------------------------------------------------

/// Random generator for column values, record batches, expressions, and plans.
/// Kotlin `class Fuzzer`.
pub struct Fuzzer {
    rng: EnhancedRandom,
}

impl Default for Fuzzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Fuzzer {
    /// Construct a fuzzer seeded with `0`, matching Kotlin's `Random(0)`.
    /// Deterministic across Rust runs.
    pub fn new() -> Self {
        Self { rng: EnhancedRandom::new(0) }
    }

    /// Generate `n` random scalar values of the given Arrow type.
    /// Kotlin: `createValues(arrowType, n) : List<Any?>`. Panics on unsupported
    /// types (matches Kotlin's `throw IllegalStateException()`).
    pub fn create_values(&mut self, data_type: &DataType, n: usize) -> Vec<ScalarValue> {
        (0..n)
            .map(|_| match data_type {
                DataType::Int8 => ScalarValue::Int8(self.rng.next_byte()),
                DataType::Int16 => ScalarValue::Int16(self.rng.next_short()),
                DataType::Int32 => ScalarValue::Int32(self.rng.next_int()),
                DataType::Int64 => ScalarValue::Int64(self.rng.next_long()),
                DataType::Float32 => ScalarValue::Float32(self.rng.next_float()),
                DataType::Float64 => ScalarValue::Float64(self.rng.next_double()),
                DataType::Utf8 => {
                    let len = self.rng.rng().gen_range(0..64);
                    ScalarValue::Utf8(self.rng.next_string(len))
                }
                other => panic!(
                    "Fuzzer::create_values: unsupported data type: {other:?}"
                ),
            })
            .collect()
    }

    /// Build a `RecordBatch` containing `n` rows of random data for every field
    /// in `schema`. Kotlin `createRecordBatch(schema, n)`. **Renamed** to avoid
    /// Rust overload conflict — see the module note.
    pub fn create_random_record_batch(
        &mut self,
        schema: &Schema,
        n: usize,
    ) -> RecordBatch {
        let columns: Vec<Vec<ScalarValue>> = schema
            .fields
            .iter()
            .map(|f| self.create_values(&f.data_type, n))
            .collect();
        self.create_record_batch(schema, columns)
    }

    /// Build a `RecordBatch` from explicit per-column data. Kotlin
    /// `createRecordBatch(schema, columns)`. Panics if `columns` is empty or any
    /// column's length disagrees with the first column's length, matching the
    /// implicit Kotlin contract (Kotlin reads `columns[0].size` as the row count).
    pub fn create_record_batch(
        &self,
        schema: &Schema,
        columns: Vec<Vec<ScalarValue>>,
    ) -> RecordBatch {
        assert!(
            !columns.is_empty(),
            "Fuzzer::create_record_batch: columns is empty"
        );
        let row_count = columns[0].len();
        let field_vectors: Vec<Box<dyn ColumnVector>> = schema
            .fields
            .iter()
            .zip(columns.into_iter())
            .map(|(field, col)| {
                assert_eq!(
                    col.len(),
                    row_count,
                    "Fuzzer::create_record_batch: column length mismatch \
                     ({} != {row_count}) for field {:?}",
                    col.len(),
                    field.name,
                );
                let mut builder = ArrowVectorBuilder::new(&field.data_type, row_count);
                for v in &col {
                    builder.append_value(v);
                }
                builder.set_value_count(row_count);
                Box::new(builder.build()) as Box<dyn ColumnVector>
            })
            .collect();
        record_batch::create(schema, field_vectors)
    }

    /// Recursively build a random logical plan tree of `project` / `filter`
    /// operators on top of `input`. Kotlin
    /// `createPlan(input, depth, maxDepth, maxExprDepth)`.
    pub fn create_plan(
        &mut self,
        input: &DataFrame,
        depth: usize,
        max_depth: usize,
        max_expr_depth: usize,
    ) -> DataFrame {
        if depth == max_depth {
            // Kotlin returns `input` directly; Rust clones because `DataFrame`
            // wraps an owned `LogicalPlan` and we need to return a fresh tree.
            return input.clone();
        }
        // Build the child plan first, then layer either a projection or a filter
        // on top of it (same shape as Kotlin's `when (rand.nextInt(2)) { ... }`).
        let child = self.create_plan(input, depth + 1, max_depth, max_expr_depth);
        match self.rng.rng().gen_range(0..2) {
            0 => {
                // 1..5 (exclusive upper bound in Rust matches Kotlin's
                // `rand.nextInt(1, 5)` which is also upper-exclusive).
                let expr_count = self.rng.rng().gen_range(1..5);
                let exprs: Vec<LogicalExpr> = (0..expr_count)
                    .map(|_| self.create_expression(&child, 0, max_expr_depth))
                    .collect();
                child.project(exprs)
            }
            _ => {
                // Note: Kotlin uses `input` here (not `child`) when generating the
                // filter predicate, so column indices reference the *original*
                // schema rather than `child`'s (which may differ after a
                // projection). Preserved verbatim for source fidelity; this is a
                // known quirk of the upstream generator.
                let pred = self.create_expression(input, 0, max_expr_depth);
                child.filter(pred)
            }
        }
    }

    /// Recursively build a random binary expression tree.
    /// Kotlin `createExpression(input, depth, maxDepth)`. Leaves at `depth ==
    /// maxDepth` are one of `ColumnIndex` / `LiteralDouble` / `LiteralLong` /
    /// `LiteralString`; internal nodes are one of the eight binary operators
    /// (`Eq` / `Neq` / `Lt` / `LtEq` / `Gt` / `GtEq` / `And` / `Or`).
    pub fn create_expression(
        &mut self,
        input: &DataFrame,
        depth: usize,
        max_depth: usize,
    ) -> LogicalExpr {
        if depth == max_depth {
            // Leaf node: pick a random literal or column reference.
            // `input.schema()` may allocate; Kotlin does the same (`input.schema().fields.size`).
            let fields_len = input.schema().fields.len();
            return match self.rng.rng().gen_range(0..4) {
                0 => LogicalExpr::ColumnIndex(self.rng.rng().gen_range(0..fields_len)),
                1 => LogicalExpr::LiteralDouble(self.rng.next_double()),
                2 => LogicalExpr::LiteralLong(self.rng.next_long()),
                _ => {
                    let len = self.rng.rng().gen_range(0..64);
                    LogicalExpr::LiteralString(self.rng.next_string(len))
                }
            };
        }
        // Internal node: binary op over two recursive subtrees.
        let l = Box::new(self.create_expression(input, depth + 1, max_depth));
        let r = Box::new(self.create_expression(input, depth + 1, max_depth));
        match self.rng.rng().gen_range(0..8) {
            0 => LogicalExpr::Eq { l, r },
            1 => LogicalExpr::Neq { l, r },
            2 => LogicalExpr::Lt { l, r },
            3 => LogicalExpr::LtEq { l, r },
            4 => LogicalExpr::Gt { l, r },
            5 => LogicalExpr::GtEq { l, r },
            6 => LogicalExpr::And { l, r },
            _ => LogicalExpr::Or { l, r },
        }
    }
}

// ---------------------------------------------------------------------------
// EnhancedRandom
// ---------------------------------------------------------------------------

/// RNG that biases toward edge-case values for each integer / floating-point
/// type (`MIN` / `MAX` / `0` / `±Inf` / `NaN`). Kotlin `class EnhancedRandom`.
///
/// Owns the underlying [`StdRng`] so that `Fuzzer` keeps a single source of
/// randomness; callers that need raw `gen_range(..)`-style calls (the Kotlin
/// `rand.nextInt(N)` sites) reach through [`Self::rng`].
pub struct EnhancedRandom {
    rng: StdRng,
}

impl EnhancedRandom {
    /// Construct seeded with the given value. `EnhancedRandom::new(0)` matches
    /// Kotlin's `Random(0)` seeding pattern.
    pub fn new(seed: u64) -> Self {
        Self { rng: StdRng::seed_from_u64(seed) }
    }

    /// Borrow the underlying RNG for direct `gen_range(..)` / `gen()` calls.
    /// This is the Rust analogue of Kotlin's `Fuzzer.rand` field; see the
    /// module note for why both APIs are exposed through a single field.
    pub fn rng(&mut self) -> &mut StdRng {
        &mut self.rng
    }

    /// Random `i8` biased toward extremes. Kotlin `nextByte()`.
    pub fn next_byte(&mut self) -> i8 {
        match self.rng.gen_range(0..5) {
            0 => i8::MIN,
            1 => i8::MAX,
            // Kotlin had two separate arms for `-0` and `0`; both reduce to the
            // same integer (0). Preserved verbatim — the bias remains identical.
            2 | 3 => 0,
            _ => self.rng.r#gen::<i32>() as i8,
        }
    }

    /// Random `i16` biased toward extremes. Kotlin `nextShort()`.
    pub fn next_short(&mut self) -> i16 {
        match self.rng.gen_range(0..5) {
            0 => i16::MIN,
            1 => i16::MAX,
            2 | 3 => 0,
            _ => self.rng.r#gen::<i32>() as i16,
        }
    }

    /// Random `i32` biased toward extremes. Kotlin `nextInt()`.
    pub fn next_int(&mut self) -> i32 {
        match self.rng.gen_range(0..5) {
            0 => i32::MIN,
            1 => i32::MAX,
            2 | 3 => 0,
            _ => self.rng.r#gen::<i32>(),
        }
    }

    /// Random `i64` biased toward extremes. Kotlin `nextLong()`.
    pub fn next_long(&mut self) -> i64 {
        match self.rng.gen_range(0..5) {
            0 => i64::MIN,
            1 => i64::MAX,
            2 | 3 => 0,
            _ => self.rng.r#gen::<i64>(),
        }
    }

    /// Random `f32` biased toward extremes including `±Inf` and `NaN`.
    /// Kotlin `nextFloat()`. Note: Kotlin uses `Float.MIN_VALUE` (smallest
    /// positive); Rust's `f32::MIN_POSITIVE` is the same value (the equivalent
    /// of Rust's `f32::MIN` would be the most-negative value, which is a
    /// different concept).
    pub fn next_float(&mut self) -> f32 {
        match self.rng.gen_range(0..8) {
            0 => f32::MIN_POSITIVE,
            1 => f32::MAX,
            2 => f32::INFINITY,
            3 => f32::NEG_INFINITY,
            4 => f32::NAN,
            5 => -0.0_f32,
            6 => 0.0_f32,
            _ => self.rng.r#gen::<f32>(),
        }
    }

    /// Random `f64` biased toward extremes including `±Inf` and `NaN`.
    /// Kotlin `nextDouble()`. See the note on `next_float` regarding
    /// `MIN_POSITIVE` vs Kotlin's `MIN_VALUE` naming.
    pub fn next_double(&mut self) -> f64 {
        match self.rng.gen_range(0..8) {
            0 => f64::MIN_POSITIVE,
            1 => f64::MAX,
            2 => f64::INFINITY,
            3 => f64::NEG_INFINITY,
            4 => f64::NAN,
            5 => -0.0_f64,
            6 => 0.0_f64,
            _ => self.rng.r#gen::<f64>(),
        }
    }

    /// Random ASCII alphanumeric string of length `len`. Kotlin `nextString(len)`.
    /// `len == 0` returns the empty string. Bytes are pulled from
    /// [`CHAR_POOL`] (`a-z` + `A-Z` + `0-9`, 62 chars).
    pub fn next_string(&mut self, len: usize) -> String {
        (0..len)
            .map(|_| CHAR_POOL[self.rng.gen_range(0..CHAR_POOL.len())] as char)
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Tests — port of `kquery/fuzzer/src/test/kotlin/FuzzerTest.kt`.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    //! The Kotlin test (`fuzzer example`) loops 50 times and `println`s each
    //! generated plan. The Rust port keeps the loop and drops the printout:
    //! the assertion is simply that `create_plan` returns without panicking
    //! 50 times in a row (i.e. random plan generation is stable across runs).
    use super::*;
    use datasource::CsvDataSource;
    use logical_plan::{LogicalPlan, Scan};
    use std::sync::Arc;

    #[test]
    fn fuzzer_example() {
        let path = "../testdata/employee.csv";
        let csv = CsvDataSource::new(path, None, true, 10);
        let input = DataFrame::new(LogicalPlan::Scan(Scan::new(
            "employee.csv",
            Arc::new(csv),
            vec![],
        )));
        let mut fuzzer = Fuzzer::new();
        for _ in 0..50 {
            // `_plan` is discarded; the test only checks that generation does
            // not panic on a depth-6 plan tree with depth-1 expressions.
            let _plan = fuzzer.create_plan(&input, 0, 6, 1);
        }
    }
}
