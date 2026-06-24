//!
//! Represents a literal value as if it were a column — every row returns the
//! same value. Used in expression evaluation when comparing column values to
//! constants.
//!
//! Translation notes:
//!   → Rust struct with `arrow_type`, `value: ScalarValue`, `size`. The `Any?`
//!   becomes `ScalarValue` per [`crate::scalar_value`] (which carries its own
//!   `Null` variant in place of `?` nullability).
//!   Rust's `Drop` is also a no-op by default. Nothing to port.

use crate::{column_vector::ColumnVector, scalar_value::ScalarValue};
use arrow_schema::DataType;

/// A column whose every row returns the same literal value.
pub struct LiteralValueVector {
    pub arrow_type: DataType,
    pub value: ScalarValue,
    pub size: usize,
}

impl LiteralValueVector {
    pub fn new(arrow_type: DataType, value: ScalarValue, size: usize) -> Self {
        Self {
            arrow_type,
            value,
            size,
        }
    }
}

impl ColumnVector for LiteralValueVector {
    fn data_type(&self) -> DataType {
        self.arrow_type.clone()
    }

    fn value(&self, i: usize) -> ScalarValue {
        if i >= self.size {
            panic!(
                "LiteralValueVector::value: index {} out of bounds (size {})",
                i, self.size
            );
        }
        self.value.clone()
    }

    fn len(&self) -> usize {
        self.size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arrow_types::INT32_TYPE;

    #[test]
    fn literal_returns_value_for_every_index() {
        let v = LiteralValueVector::new(INT32_TYPE, ScalarValue::Int32(42), 5);
        assert_eq!(v.len(), 5);
        for i in 0..5 {
            assert_eq!(v.value(i), ScalarValue::Int32(42));
        }
    }

    #[test]
    fn literal_data_type_matches_constructor_arg() {
        let v = LiteralValueVector::new(INT32_TYPE, ScalarValue::Int32(7), 3);
        assert_eq!(v.data_type(), INT32_TYPE);
    }

    #[test]
    #[should_panic(expected = "out of bounds")]
    fn literal_index_out_of_bounds_panics() {
        let v = LiteralValueVector::new(INT32_TYPE, ScalarValue::Int32(1), 2);
        let _ = v.value(2);
    }
}
