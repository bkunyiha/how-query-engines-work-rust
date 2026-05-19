//! Port of `kquery/datatypes/src/main/kotlin/ArrowTypes.kt`.
//!
//! Named Arrow `DataType` constants — the type vocabulary the engine uses
//! throughout. The Kotlin original is a singleton `object ArrowTypes { val
//! BooleanType = ArrowType.Bool() ... }`. We translate it to a Rust module
//! of `pub const` values backed by `arrow_schema::DataType`. The names match
//! the Kotlin source 1:1.
//!
//! Translation notes:
//! - Kotlin `object` (singleton) → Rust `mod` with `pub const`. Same call-site
//!   ergonomics: `ArrowTypes.Int32Type` (Kotlin) becomes `arrow_types::INT32_TYPE`
//!   (Rust) or — via re-export — `ArrowTypes::INT32_TYPE` if a unit struct is
//!   used. We chose the module-of-constants form because `DataType` variants
//!   are `const`-constructible and a unit struct would add no value.
//! - Naming: Kotlin `Int32Type` (camelCase) → Rust `INT32_TYPE` (SCREAMING_SNAKE_CASE)
//!   per Rust convention for constants. Type names elsewhere (e.g. `Schema`,
//!   `ArrowFieldVector`) keep PascalCase by Rust convention.

use arrow_schema::{DataType, IntervalUnit, TimeUnit};

pub const BOOLEAN_TYPE: DataType = DataType::Boolean;

pub const INT8_TYPE:  DataType = DataType::Int8;
pub const INT16_TYPE: DataType = DataType::Int16;
pub const INT32_TYPE: DataType = DataType::Int32;
pub const INT64_TYPE: DataType = DataType::Int64;

pub const UINT8_TYPE:  DataType = DataType::UInt8;
pub const UINT16_TYPE: DataType = DataType::UInt16;
pub const UINT32_TYPE: DataType = DataType::UInt32;
pub const UINT64_TYPE: DataType = DataType::UInt64;

pub const FLOAT_TYPE:  DataType = DataType::Float32;
pub const DOUBLE_TYPE: DataType = DataType::Float64;

pub const STRING_TYPE: DataType = DataType::Utf8;
pub const BINARY_TYPE: DataType = DataType::Binary;

pub const DATE_DAY_TYPE: DataType = DataType::Date32;

// Kotlin's `IntervalDayTime` becomes arrow-rs's `DataType::Interval(IntervalUnit::DayTime)`.
// IntervalUnit is not const-constructible inside a const context in some arrow versions;
// expose this as a `pub fn` returning the value if `const` evaluation fails to compile,
// but in arrow 55 the variant *is* const-friendly.
pub const INTERVAL_DAY_TIME_TYPE: DataType = DataType::Interval(IntervalUnit::DayTime);

// Convenience — matches Kotlin's missing-but-implied timestamp type. Kept here so
// downstream crates have a single import path for every commonly-used DataType.
pub const TIMESTAMP_MICRO_TYPE: DataType = DataType::Timestamp(TimeUnit::Microsecond, None);
