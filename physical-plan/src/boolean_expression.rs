//! Port of `kquery/physical-plan/src/main/kotlin/expressions/BooleanExpression.kt`.
//!
//! Comparison and logical operators that always produce a `Boolean` column:
//! `AND`, `OR`, `=`, `!=`, `<`, `<=`, `>`, `>=`.
//!
//! ## Translation note — abstract class → trait with default method
//! As with [`crate::binary_expression::BinaryExpression`], the Kotlin
//! `abstract class BooleanExpression` becomes a **trait with a default method**.
//! Unlike the math family, `BooleanExpression` extends [`Expression`] directly
//! (not `BinaryExpression`): comparison does *not* coerce numeric types — it
//! requires the two sides to already share a type and panics otherwise. The
//! default [`BooleanExpression::evaluate_boolean`] holds the shared "evaluate
//! both sides, build a Boolean column cell-by-cell" logic; each concrete operator
//! supplies only its per-cell predicate via `compare_value`.
//!
//! ## Translation note — the `compare_typed!` macro
//! The Kotlin comparison classes are six near-identical `when (arrowType)` blocks
//! that differ only in the operator (`==`, `<`, `>`, …). Rather than copy that
//! eight-arm match six times, [`compare_typed!`] is a small `macro_rules!` macro
//! parameterised by the operator token. `compare_typed!(l, r, t, >=)` expands to
//! the full per-type match applying `>=` to the typed values pulled out of each
//! `ScalarValue`. Using the real Rust operator per type also gives the correct
//! `NaN` behaviour for free (`NaN >= x` is `false`), matching the JVM.

use crate::expressions::{as_date, as_f32, as_f64, as_i16, as_i32, as_i64, as_i8, as_str, Expression};
use arrow_schema::DataType;
use datatypes::arrow_types::BOOLEAN_TYPE;
use datatypes::{ArrowVectorBuilder, ColumnVector, RecordBatch, ScalarValue};
use std::sync::Arc;

/// A boolean (comparison or logical) binary expression. Kotlin
/// `abstract class BooleanExpression`.
pub trait BooleanExpression: Expression {
    /// The left operand expression.
    fn left(&self) -> &Arc<dyn Expression>;
    /// The right operand expression.
    fn right(&self) -> &Arc<dyn Expression>;

    /// The per-cell predicate. Kotlin: the abstract
    /// `evaluate(l: Any?, r: Any?, arrowType: ArrowType): Boolean`.
    fn compare_value(&self, l: &ScalarValue, r: &ScalarValue, arrow_type: &DataType) -> bool;

    /// Template method (Kotlin `evaluate(input)` + `compare`): evaluate both
    /// sides, require equal lengths and identical types, then build a `Boolean`
    /// column by applying [`compare_value`](Self::compare_value) cell-by-cell.
    fn evaluate_boolean(&self, input: &RecordBatch) -> Box<dyn ColumnVector> {
        let ll = self.left().evaluate(input);
        let rr = self.right().evaluate(input);
        assert_eq!(ll.size(), rr.size());
        if ll.get_type() != rr.get_type() {
            panic!(
                "Cannot compare values of different type: {:?} != {:?}",
                ll.get_type(),
                rr.get_type()
            );
        }
        let arrow_type = ll.get_type();
        let mut builder = ArrowVectorBuilder::new(&BOOLEAN_TYPE, ll.size());
        for i in 0..ll.size() {
            let value = self.compare_value(&ll.get_value(i), &rr.get_value(i), &arrow_type);
            builder.append_value(&ScalarValue::Boolean(value));
        }
        builder.set_value_count(ll.size());
        Box::new(builder.build())
    }
}

/// Expand to a per-type `match` that applies the comparison operator `$op` to the
/// two operands, pulling the typed value out of each `ScalarValue`. The supported
/// types mirror the Kotlin `when (arrowType)` arms.
macro_rules! compare_typed {
    ($l:expr, $r:expr, $t:expr, $op:tt) => {
        match $t {
            DataType::Int8 => as_i8($l) $op as_i8($r),
            DataType::Int16 => as_i16($l) $op as_i16($r),
            DataType::Int32 => as_i32($l) $op as_i32($r),
            DataType::Int64 => as_i64($l) $op as_i64($r),
            DataType::Float32 => as_f32($l) $op as_f32($r),
            DataType::Float64 => as_f64($l) $op as_f64($r),
            DataType::Utf8 => as_str($l) $op as_str($r),
            DataType::Date32 => as_date($l) $op as_date($r),
            other => panic!("Unsupported data type in comparison expression: {other:?}"),
        }
    };
}

/// Kotlin `toBool`: a boolean is itself; a number is "true" iff it equals 1.
/// Logical `AND`/`OR` apply this to each operand. In practice the operands are
/// already Boolean columns (the results of comparisons).
fn to_bool(v: &ScalarValue) -> bool {
    match v {
        ScalarValue::Boolean(b) => *b,
        ScalarValue::Int8(n) => *n == 1,
        ScalarValue::Int16(n) => *n == 1,
        ScalarValue::Int32(n) => *n == 1,
        ScalarValue::Int64(n) => *n == 1,
        ScalarValue::UInt8(n) => *n == 1,
        ScalarValue::UInt16(n) => *n == 1,
        ScalarValue::UInt32(n) => *n == 1,
        ScalarValue::UInt64(n) => *n == 1,
        other => panic!("Cannot convert {other:?} to bool"),
    }
}

/// Generate a concrete boolean operator: a struct holding `l`/`r`, its
/// `BooleanExpression` impl (`compare_value` is the per-cell body `$body`), the
/// trivial `Expression` delegate to `evaluate_boolean`, and a `Display` impl
/// rendering `"l <sym> r"`. This mirrors the Kotlin file, where each operator is
/// a tiny class overriding one method.
macro_rules! boolean_op {
    ($name:ident, $sym:literal, |$l:ident, $r:ident, $t:ident| $body:expr) => {
        #[doc = concat!("`l ", $sym, " r`. Kotlin `", stringify!($name), "`.")]
        pub struct $name {
            l: Arc<dyn Expression>,
            r: Arc<dyn Expression>,
        }

        impl $name {
            pub fn new(l: Arc<dyn Expression>, r: Arc<dyn Expression>) -> Self {
                Self { l, r }
            }
        }

        impl BooleanExpression for $name {
            fn left(&self) -> &Arc<dyn Expression> {
                &self.l
            }
            fn right(&self) -> &Arc<dyn Expression> {
                &self.r
            }
            fn compare_value(
                &self,
                $l: &ScalarValue,
                $r: &ScalarValue,
                $t: &DataType,
            ) -> bool {
                $body
            }
        }

        impl Expression for $name {
            fn evaluate(&self, input: &RecordBatch) -> Box<dyn ColumnVector> {
                self.evaluate_boolean(input)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{} {} {}", self.l, $sym, self.r)
            }
        }
    };
}

// AND / OR ignore the Arrow type and operate on the truthiness of each side.
boolean_op!(AndExpression, "AND", |l, r, _t| to_bool(l) && to_bool(r));
boolean_op!(OrExpression, "OR", |l, r, _t| to_bool(l) || to_bool(r));

// Comparisons dispatch on the (shared) Arrow type via `compare_typed!`.
boolean_op!(EqExpression, "=", |l, r, t| compare_typed!(l, r, t, ==));
boolean_op!(NeqExpression, "!=", |l, r, t| compare_typed!(l, r, t, !=));
boolean_op!(LtExpression, "<", |l, r, t| compare_typed!(l, r, t, <));
boolean_op!(LtEqExpression, "<=", |l, r, t| compare_typed!(l, r, t, <=));
boolean_op!(GtExpression, ">", |l, r, t| compare_typed!(l, r, t, >));
boolean_op!(GtEqExpression, ">=", |l, r, t| compare_typed!(l, r, t, >=));

#[cfg(test)]
mod tests {
    //! Port of `kquery/physical-plan/src/test/kotlin/BooleanExpressionTest.kt`.
    //!
    //! The Kotlin tests build their input batch with `Fuzzer().createRecordBatch`.
    //! The `fuzzer` crate is module 9 and is not yet ported, so these tests build
    //! the `RecordBatch` directly from typed arrow arrays — exactly what
    //! `createRecordBatch` does internally. Each test compares the operator's
    //! output against Rust's own `>=` over the same values, so the assertion is
    //! self-consistent regardless of the concrete numbers chosen.
    use super::*;
    use crate::column_expression::ColumnExpression;
    use arrow_array::{
        ArrayRef, Float64Array, Int16Array, Int32Array, Int64Array, Int8Array, StringArray,
    };
    use arrow_schema::{Field as ArrowField, Schema as ArrowSchema};
    use datatypes::RecordBatch;
    use std::sync::Arc;

    /// Build a two-column batch ("a", "b") of the same type.
    fn batch2(t: DataType, a: ArrayRef, b: ArrayRef) -> RecordBatch {
        let schema = Arc::new(ArrowSchema::new(vec![
            ArrowField::new("a", t.clone(), true),
            ArrowField::new("b", t, true),
        ]));
        RecordBatch::try_new(schema, vec![a, b]).unwrap()
    }

    fn gteq(batch: &RecordBatch) -> Box<dyn ColumnVector> {
        let expr = GtEqExpression::new(
            Arc::new(ColumnExpression::new(0)),
            Arc::new(ColumnExpression::new(1)),
        );
        expr.evaluate(batch)
    }

    #[test]
    fn gteq_bytes() {
        let a: Vec<i8> = vec![10, 20, 30, i8::MIN, i8::MAX];
        let b: Vec<i8> = vec![10, 30, 20, i8::MAX, i8::MIN];
        let batch = batch2(
            DataType::Int8,
            Arc::new(Int8Array::from(a.clone())),
            Arc::new(Int8Array::from(b.clone())),
        );
        let result = gteq(&batch);
        assert_eq!(result.size(), a.len());
        for i in 0..result.size() {
            assert_eq!(result.get_value(i), ScalarValue::Boolean(a[i] >= b[i]));
        }
    }

    #[test]
    fn gteq_shorts() {
        let a: Vec<i16> = vec![111, 222, 333, i16::MIN, i16::MAX];
        let b: Vec<i16> = vec![111, 333, 222, i16::MAX, i16::MIN];
        let batch = batch2(
            DataType::Int16,
            Arc::new(Int16Array::from(a.clone())),
            Arc::new(Int16Array::from(b.clone())),
        );
        let result = gteq(&batch);
        assert_eq!(result.size(), a.len());
        for i in 0..result.size() {
            assert_eq!(result.get_value(i), ScalarValue::Boolean(a[i] >= b[i]));
        }
    }

    #[test]
    fn gteq_ints() {
        let a: Vec<i32> = vec![111, 222, 333, i32::MIN, i32::MAX];
        let b: Vec<i32> = vec![111, 333, 222, i32::MAX, i32::MIN];
        let batch = batch2(
            DataType::Int32,
            Arc::new(Int32Array::from(a.clone())),
            Arc::new(Int32Array::from(b.clone())),
        );
        let result = gteq(&batch);
        assert_eq!(result.size(), a.len());
        for i in 0..result.size() {
            assert_eq!(result.get_value(i), ScalarValue::Boolean(a[i] >= b[i]));
        }
    }

    #[test]
    fn gteq_longs() {
        let a: Vec<i64> = vec![111, 222, 333, i64::MIN, i64::MAX];
        let b: Vec<i64> = vec![111, 333, 222, i64::MAX, i64::MIN];
        let batch = batch2(
            DataType::Int64,
            Arc::new(Int64Array::from(a.clone())),
            Arc::new(Int64Array::from(b.clone())),
        );
        let result = gteq(&batch);
        assert_eq!(result.size(), a.len());
        for i in 0..result.size() {
            assert_eq!(result.get_value(i), ScalarValue::Boolean(a[i] >= b[i]));
        }
    }

    #[test]
    fn gteq_doubles() {
        // Includes NaN: `NaN >= x` is false on both sides, so the operator's
        // output and Rust's own `>=` agree.
        let a: Vec<f64> = vec![0.0, 1.0, f64::MIN_POSITIVE, f64::MAX, f64::NAN];
        let b: Vec<f64> = a.iter().copied().rev().collect();
        let batch = batch2(
            DataType::Float64,
            Arc::new(Float64Array::from(a.clone())),
            Arc::new(Float64Array::from(b.clone())),
        );
        let result = gteq(&batch);
        assert_eq!(result.size(), a.len());
        for i in 0..result.size() {
            assert_eq!(result.get_value(i), ScalarValue::Boolean(a[i] >= b[i]));
        }
    }

    #[test]
    fn gteq_strings() {
        let a = vec!["aaa", "bbb", "ccc"];
        let b = vec!["aaa", "ccc", "bbb"];
        let batch = batch2(
            DataType::Utf8,
            Arc::new(StringArray::from(a.clone())),
            Arc::new(StringArray::from(b.clone())),
        );
        let result = gteq(&batch);
        assert_eq!(result.size(), a.len());
        for i in 0..result.size() {
            assert_eq!(result.get_value(i), ScalarValue::Boolean(a[i] >= b[i]));
        }
    }
}
