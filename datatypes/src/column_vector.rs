//! Port of `kquery/datatypes/src/main/kotlin/ColumnVector.kt`.
//!
//! Abstraction over different implementations of a column vector.
//!
//! Translation notes:
//! - Kotlin `interface ColumnVector` → Rust `pub trait ColumnVector`.
//! - Kotlin's `AutoCloseable` parent interface is *not* ported. arrow-rs
//!   uses reference-counted `Arc<dyn Array>` for its array types, so explicit
//!   `close()` is unnecessary — memory is released when the last `Arc` is
//!   dropped. The `Drop` trait handles any non-Arrow resources automatically.
//! - Kotlin `getValue(i: Int): Any?` becomes `get_value(i: usize) -> ScalarValue`
//!   as the typed-enum substitution for `Any`. The
//!   `Null` variant of [`ScalarValue`] carries the nullability that Kotlin
//!   expressed with the `?` (nullable) marker.
//! - Method renaming: `getType` / `getValue` → `get_type` / `get_value` per
//!   Rust convention.
//!
//! [`ScalarValue`]: crate::scalar_value::ScalarValue

use crate::scalar_value::ScalarValue;
use arrow_schema::DataType;

/// Abstraction over different implementations of a column vector.
pub trait ColumnVector {
    /// The Arrow data type stored in this column.
    fn get_type(&self) -> DataType;

    /// Fetch one cell by row index. Returns [`ScalarValue::Null`] for null cells.
    fn get_value(&self, i: usize) -> ScalarValue;

    /// Number of rows in this column.
    fn size(&self) -> usize;
}
