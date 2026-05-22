//! Port of `kquery/logical-plan/src/main/kotlin/Expressions.kt`.
//!
//! # What lives here vs. in `logical_expr.rs`
//!
//! `Expressions.kt` held three kinds of thing: (1) the concrete `LogicalExpr`
//! implementing classes, (2) the `AggregateExpr` family, and (3) the convenience
//! constructors. The port routes each to the file where it belongs:
//!
//! - (1) The plain `LogicalExpr` implementors are the SUMMANDS of the
//!   `LogicalExpr` sum type. A sum type *is* its summands, so they collapse into
//!   the enum in `logical_expr.rs` and are NOT here.
//!
//! - (2) `AggregateExpr` is its **own** sum type and lives here. In Kotlin it is
//!   `abstract class AggregateExpr : LogicalExpr` — a *narrow* family that is
//!   also part of the broad `LogicalExpr` family. Rust `enum`s have no
//!   inheritance, so the port keeps `AggregateExpr` as a distinct enum (the
//!   `Aggregate` plan ranges over a typed `Vec<AggregateExpr>`) and bridges it
//!   into `LogicalExpr` with the single `LogicalExpr::AggregateExpr` variant —
//!   exactly the shape of DataFusion's `Expr::AggregateFunction`. The
//!   `From<AggregateExpr> for LogicalExpr` impl below *is* that bridge.
//!
//! - (3) The convenience constructors are *introduction forms* — functions INTO
//!   a type (`col: &str -> LogicalExpr`, `lit_long: i64 -> LogicalExpr`, the
//!   `eq`/`add`/… builder methods, and `sum`/`min`/… which build an
//!   `AggregateExpr`). An arrow that merely *targets* a type is separable from
//!   that type's definition, so they live here.
//!
//! Spelling shifts: Kotlin uses overloaded `lit(...)` and infix operators
//! (`a eq b`). Rust has neither, so the literal constructors are spelled out
//! (`lit_string`, `lit_long`, …) and the infix operators become `self`-consuming
//! methods (`a.eq(b)`, `a.mult(b).alias("x")`).

use crate::logical_expr::LogicalExpr;
use crate::logical_plan::LogicalPlan;
use arrow_schema::DataType;
use datatypes::arrow_types::{INT32_TYPE, UINT32_TYPE};
use datatypes::Field;
use std::fmt;

/// Aggregate functions. Kotlin: `abstract class AggregateExpr : LogicalExpr`
/// with the `Sum` / `Min` / `Max` / `Avg` / `Count` / `CountDistinct`
/// subclasses. Kept as its own enum so the `Aggregate` plan and
/// `DataFrame::aggregate` keep a typed `Vec<AggregateExpr>`; bridged into
/// `LogicalExpr` (for nesting inside expressions, e.g. `HAVING`) by the
/// `From<AggregateExpr> for LogicalExpr` impl below — the analogue of Kotlin's
/// `AggregateExpr : LogicalExpr` and of DataFusion's `Expr::AggregateFunction`.
#[derive(Debug, Clone, PartialEq)]
pub enum AggregateExpr {
    /// Kotlin `Sum`.
    Sum(LogicalExpr),
    /// Kotlin `Min`.
    Min(LogicalExpr),
    /// Kotlin `Max`.
    Max(LogicalExpr),
    /// Kotlin `Avg`.
    Avg(LogicalExpr),
    /// Kotlin `Count`.
    Count(LogicalExpr),
    /// Kotlin `CountDistinct`.
    CountDistinct(LogicalExpr),
}

impl AggregateExpr {
    /// Kotlin: `AggregateExpr.toField(input)`. SUM/MIN/MAX/AVG carry the data
    /// type of their input expression; COUNT and COUNT DISTINCT are integer
    /// counts.
    pub fn to_field(&self, input: &LogicalPlan) -> Field {
        match self {
            AggregateExpr::Sum(e) => Field::new("SUM", e.to_field(input).data_type),
            AggregateExpr::Min(e) => Field::new("MIN", e.to_field(input).data_type),
            AggregateExpr::Max(e) => Field::new("MAX", e.to_field(input).data_type),
            AggregateExpr::Avg(e) => Field::new("AVG", e.to_field(input).data_type),
            AggregateExpr::Count(_) => Field::new("COUNT", INT32_TYPE),
            AggregateExpr::CountDistinct(_) => Field::new("COUNT_DISTINCT", UINT32_TYPE),
        }
    }
}

impl fmt::Display for AggregateExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AggregateExpr::Sum(e) => write!(f, "SUM({e})"),
            AggregateExpr::Min(e) => write!(f, "MIN({e})"),
            AggregateExpr::Max(e) => write!(f, "MAX({e})"),
            AggregateExpr::Avg(e) => write!(f, "AVG({e})"),
            AggregateExpr::Count(e) => write!(f, "COUNT({e})"),
            AggregateExpr::CountDistinct(e) => write!(f, "COUNT(DISTINCT {e})"),
        }
    }
}

/// The bridge: inject an `AggregateExpr` into `LogicalExpr` so it can nest
/// inside any expression. Kotlin gets this free via `AggregateExpr : LogicalExpr`;
/// the Rust port spells it out (cf. DataFusion's `Expr::AggregateFunction`).
impl From<AggregateExpr> for LogicalExpr {
    fn from(agg: AggregateExpr) -> Self {
        LogicalExpr::AggregateExpr(Box::new(agg))
    }
}

// ==============================================================
// Infix-operator equivalents — Kotlin `infix fun LogicalExpr.eq(rhs)` etc.
// become `self`-consuming builder methods.
// ==============================================================
// `add` / `div` deliberately mirror the Kotlin infix builder names (alongside
// `subtract` / `mult` / `modulus`); they build AST nodes, not compute values,
// so they are intentionally *not* `std::ops::{Add, Div}` impls.
#[allow(clippy::should_implement_trait)]
impl LogicalExpr {
    pub fn eq(self, rhs: LogicalExpr) -> LogicalExpr {
        LogicalExpr::Eq { l: Box::new(self), r: Box::new(rhs) }
    }
    pub fn neq(self, rhs: LogicalExpr) -> LogicalExpr {
        LogicalExpr::Neq { l: Box::new(self), r: Box::new(rhs) }
    }
    pub fn gt(self, rhs: LogicalExpr) -> LogicalExpr {
        LogicalExpr::Gt { l: Box::new(self), r: Box::new(rhs) }
    }
    pub fn gteq(self, rhs: LogicalExpr) -> LogicalExpr {
        LogicalExpr::GtEq { l: Box::new(self), r: Box::new(rhs) }
    }
    pub fn lt(self, rhs: LogicalExpr) -> LogicalExpr {
        LogicalExpr::Lt { l: Box::new(self), r: Box::new(rhs) }
    }
    pub fn lteq(self, rhs: LogicalExpr) -> LogicalExpr {
        LogicalExpr::LtEq { l: Box::new(self), r: Box::new(rhs) }
    }
    pub fn and(self, rhs: LogicalExpr) -> LogicalExpr {
        LogicalExpr::And { l: Box::new(self), r: Box::new(rhs) }
    }
    pub fn or(self, rhs: LogicalExpr) -> LogicalExpr {
        LogicalExpr::Or { l: Box::new(self), r: Box::new(rhs) }
    }
    pub fn add(self, rhs: LogicalExpr) -> LogicalExpr {
        LogicalExpr::Add { l: Box::new(self), r: Box::new(rhs) }
    }
    pub fn subtract(self, rhs: LogicalExpr) -> LogicalExpr {
        LogicalExpr::Subtract { l: Box::new(self), r: Box::new(rhs) }
    }
    pub fn mult(self, rhs: LogicalExpr) -> LogicalExpr {
        LogicalExpr::Multiply { l: Box::new(self), r: Box::new(rhs) }
    }
    pub fn div(self, rhs: LogicalExpr) -> LogicalExpr {
        LogicalExpr::Divide { l: Box::new(self), r: Box::new(rhs) }
    }
    pub fn modulus(self, rhs: LogicalExpr) -> LogicalExpr {
        LogicalExpr::Modulus { l: Box::new(self), r: Box::new(rhs) }
    }
    pub fn alias(self, alias: impl Into<String>) -> LogicalExpr {
        LogicalExpr::Alias { expr: Box::new(self), alias: alias.into() }
    }
}

// ==============================================================
// Convenience constructors — Kotlin top-level `fun col`, `fun lit`, `fun cast`,
// `fun max`, etc.
// ==============================================================

/// Create a column reference by name. Kotlin: `fun col(name)`.
pub fn col(name: impl Into<String>) -> LogicalExpr {
    LogicalExpr::Column(name.into())
}

/// Kotlin: `fun lit(value: String)`.
pub fn lit_string(value: impl Into<String>) -> LogicalExpr {
    LogicalExpr::LiteralString(value.into())
}
/// Kotlin: `fun lit(value: Long)`.
pub fn lit_long(value: i64) -> LogicalExpr {
    LogicalExpr::LiteralLong(value)
}
/// Kotlin: `fun lit(value: Float)`.
pub fn lit_float(value: f32) -> LogicalExpr {
    LogicalExpr::LiteralFloat(value)
}
/// Kotlin: `fun lit(value: Double)`.
pub fn lit_double(value: f64) -> LogicalExpr {
    LogicalExpr::LiteralDouble(value)
}
/// Kotlin: `fun lit(value: LocalDate)`. Takes the ISO-8601 text form.
pub fn lit_date(value: impl Into<String>) -> LogicalExpr {
    LogicalExpr::LiteralDate(value.into())
}

/// Kotlin: `fun cast(expr, dataType)`.
pub fn cast(expr: LogicalExpr, data_type: DataType) -> LogicalExpr {
    LogicalExpr::Cast { expr: Box::new(expr), data_type }
}

/// Kotlin: `Sum(expr)`.
pub fn sum(expr: LogicalExpr) -> AggregateExpr {
    AggregateExpr::Sum(expr)
}
/// Kotlin: `Min(expr)`.
pub fn min(expr: LogicalExpr) -> AggregateExpr {
    AggregateExpr::Min(expr)
}
/// Kotlin: `fun max(expr) = Max(expr)`.
pub fn max(expr: LogicalExpr) -> AggregateExpr {
    AggregateExpr::Max(expr)
}
/// Kotlin: `Avg(expr)`.
pub fn avg(expr: LogicalExpr) -> AggregateExpr {
    AggregateExpr::Avg(expr)
}
/// Kotlin: `Count(expr)`.
pub fn count(expr: LogicalExpr) -> AggregateExpr {
    AggregateExpr::Count(expr)
}
/// Kotlin: `CountDistinct(expr)`.
pub fn count_distinct(expr: LogicalExpr) -> AggregateExpr {
    AggregateExpr::CountDistinct(expr)
}
