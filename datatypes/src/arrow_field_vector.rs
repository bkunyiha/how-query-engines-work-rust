//! Port of `kquery/datatypes/src/main/kotlin/ArrowFieldVector.kt`.
//!
//! Wraps an arrow-rs `ArrayRef` so it implements [`ColumnVector`]. The Kotlin
//! source has three things in this file:
//! 1. `ArrowAllocator` (singleton wrapping a `RootAllocator`)
//! 2. `FieldVectorFactory.create(...)` — builds typed `FieldVector`s
//! 3. `ArrowFieldVector` class — wraps a `FieldVector`, implements `ColumnVector`
//!
//! Translation notes:
//! - **`ArrowAllocator` is NOT ported.** arrow-rs uses its own memory model
//!   (`Buffer` is `Arc`-backed) and does not expose a separate allocator
//!   abstraction at this level. Vector construction in arrow-rs goes through
//!   typed builders (`Int32Builder::new()`, etc.) that allocate directly.
//! - **`FieldVectorFactory.create(...)`** is also not ported as-is. The Kotlin
//!   factory returns a `FieldVector` ready to be mutated; arrow-rs's equivalent
//!   is the per-type builder pattern handled in [`crate::arrow_vector_builder`].
//! - **`ArrowFieldVector`** wraps `arrow_array::ArrayRef` (= `Arc<dyn Array>`)
//!   instead of Java Arrow's `FieldVector`. The `get_type`/`get_value`/`size`
//!   methods dispatch on the underlying Arrow type — the Rust version uses
//!   `array.as_any().downcast_ref::<...>()` where Kotlin used `is`-checks.

use crate::{column_vector::ColumnVector, scalar_value::ScalarValue};
use arrow_array::{
    Array, ArrayRef, BinaryArray, BooleanArray, Date32Array, Float32Array, Float64Array,
    Int16Array, Int32Array, Int64Array, Int8Array, StringArray, UInt16Array, UInt32Array,
    UInt64Array, UInt8Array,
};
use arrow_schema::DataType;

/// Wrapper around an arrow-rs `ArrayRef` (`Arc<dyn Array>`) implementing
/// [`ColumnVector`]. Renamed from Kotlin's `field: FieldVector` to
/// `field: ArrayRef` because arrow-rs uses `Array`, not `FieldVector`,
/// as its trait object.
pub struct ArrowFieldVector {
    pub field: ArrayRef,
}

impl ArrowFieldVector {
    pub fn new(field: ArrayRef) -> Self {
        Self { field }
    }
}

impl ColumnVector for ArrowFieldVector {
    fn get_type(&self) -> DataType {
        // arrow-rs's `Array` trait carries the data type directly — no need
        // to dispatch on concrete vector types as the Kotlin version does.
        self.field.data_type().clone()
    }

    fn get_value(&self, i: usize) -> ScalarValue {
        if self.field.is_null(i) {
            return ScalarValue::Null;
        }
        // Dispatch on the data type, then downcast to the concrete array
        // implementation to read the typed value. This is the Rust analogue
        // of Kotlin's `when (field) { is BitVector -> ..., is IntVector -> ... }`.
        match self.field.data_type() {
            DataType::Boolean => {
                let a = self.field.as_any().downcast_ref::<BooleanArray>().unwrap();
                ScalarValue::Boolean(a.value(i))
            }
            DataType::Int8 => {
                let a = self.field.as_any().downcast_ref::<Int8Array>().unwrap();
                ScalarValue::Int8(a.value(i))
            }
            DataType::Int16 => {
                let a = self.field.as_any().downcast_ref::<Int16Array>().unwrap();
                ScalarValue::Int16(a.value(i))
            }
            DataType::Int32 => {
                let a = self.field.as_any().downcast_ref::<Int32Array>().unwrap();
                ScalarValue::Int32(a.value(i))
            }
            DataType::Int64 => {
                let a = self.field.as_any().downcast_ref::<Int64Array>().unwrap();
                ScalarValue::Int64(a.value(i))
            }
            DataType::UInt8 => {
                let a = self.field.as_any().downcast_ref::<UInt8Array>().unwrap();
                ScalarValue::UInt8(a.value(i))
            }
            DataType::UInt16 => {
                let a = self.field.as_any().downcast_ref::<UInt16Array>().unwrap();
                ScalarValue::UInt16(a.value(i))
            }
            DataType::UInt32 => {
                let a = self.field.as_any().downcast_ref::<UInt32Array>().unwrap();
                ScalarValue::UInt32(a.value(i))
            }
            DataType::UInt64 => {
                let a = self.field.as_any().downcast_ref::<UInt64Array>().unwrap();
                ScalarValue::UInt64(a.value(i))
            }
            DataType::Float32 => {
                let a = self.field.as_any().downcast_ref::<Float32Array>().unwrap();
                ScalarValue::Float32(a.value(i))
            }
            DataType::Float64 => {
                let a = self.field.as_any().downcast_ref::<Float64Array>().unwrap();
                ScalarValue::Float64(a.value(i))
            }
            DataType::Utf8 => {
                let a = self.field.as_any().downcast_ref::<StringArray>().unwrap();
                ScalarValue::Utf8(a.value(i).to_string())
            }
            DataType::Binary => {
                let a = self.field.as_any().downcast_ref::<BinaryArray>().unwrap();
                ScalarValue::Binary(a.value(i).to_vec())
            }
            DataType::Date32 => {
                let a = self.field.as_any().downcast_ref::<Date32Array>().unwrap();
                ScalarValue::Date32(a.value(i))
            }
            other => panic!("ArrowFieldVector::get_value: unsupported data type: {:?}", other),
        }
    }

    fn size(&self) -> usize {
        self.field.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn int32_round_trip() {
        let arr: ArrayRef = Arc::new(Int32Array::from(vec![1, 2, 3]));
        let v = ArrowFieldVector::new(arr);
        assert_eq!(v.size(), 3);
        assert_eq!(v.get_type(), DataType::Int32);
        assert_eq!(v.get_value(0), ScalarValue::Int32(1));
        assert_eq!(v.get_value(2), ScalarValue::Int32(3));
    }

    #[test]
    fn nullability_returns_scalar_null() {
        let arr: ArrayRef = Arc::new(Int32Array::from(vec![Some(7), None, Some(9)]));
        let v = ArrowFieldVector::new(arr);
        assert_eq!(v.get_value(0), ScalarValue::Int32(7));
        assert_eq!(v.get_value(1), ScalarValue::Null);
        assert_eq!(v.get_value(2), ScalarValue::Int32(9));
    }

    #[test]
    fn utf8_round_trip() {
        let arr: ArrayRef = Arc::new(StringArray::from(vec!["a", "bb", "ccc"]));
        let v = ArrowFieldVector::new(arr);
        assert_eq!(v.get_value(1), ScalarValue::Utf8("bb".to_string()));
    }

    #[test]
    fn boolean_round_trip() {
        let arr: ArrayRef = Arc::new(BooleanArray::from(vec![true, false, true]));
        let v = ArrowFieldVector::new(arr);
        assert_eq!(v.get_value(0), ScalarValue::Boolean(true));
        assert_eq!(v.get_value(1), ScalarValue::Boolean(false));
    }
}
