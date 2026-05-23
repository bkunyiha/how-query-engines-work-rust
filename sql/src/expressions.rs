//! Port of `kquery/sql/src/main/kotlin/Expressions.kt`.
//!
//! The SQL **AST** produced by the parser. This is distinct from the
//! logical-plan `LogicalExpr`: `SqlPlanner` translates this untyped, pre-binding
//! syntax tree into the schema-aware logical expressions.
//!
//! ## Translation notes
//! - Kotlin's marker `interface SqlExpr` plus its implementing data classes
//!   collapse into one `SqlExpr` enum (interface-hierarchy → enum, §3.1).
//! - Kotlin's `interface SqlRelation : SqlExpr` is a *marker* sub-interface
//!   (no methods) with a single implementor, `SqlSelect`. The marker carries no
//!   behaviour, so it is not reproduced; `SqlSelect` is a struct surfaced as the
//!   `SqlExpr::Select` variant.
//! - Kotlin's `SqlAlias(expr, alias: SqlIdentifier)` and
//!   `SqlCast(expr, dataType: SqlIdentifier)` store a `SqlIdentifier`. Those
//!   fields are *always* identifiers, and the planner reads only the inner
//!   string (`.id`), so the Rust port flattens them to `String`.

/// A SQL abstract-syntax-tree node. Kotlin: `interface SqlExpr` + implementors.
#[derive(Debug, Clone, PartialEq)]
pub enum SqlExpr {
    /// Identifier — table/column name. Kotlin `SqlIdentifier(id)`.
    Identifier(String),
    /// Binary expression. Kotlin `SqlBinaryExpr(l, op, r)`.
    BinaryExpr { l: Box<SqlExpr>, op: String, r: Box<SqlExpr> },
    /// String literal. Kotlin `SqlString(value)`.
    String(String),
    /// Long literal. Kotlin `SqlLong(value)`.
    Long(i64),
    /// Double literal. Kotlin `SqlDouble(value)`.
    Double(f64),
    /// Date literal. Kotlin `SqlDate(value)`.
    Date(String),
    /// Interval literal. Kotlin `SqlInterval(value)`.
    Interval(String),
    /// Function call. Kotlin `SqlFunction(id, args)`.
    Function { id: String, args: Vec<SqlExpr> },
    /// Aliased expression `expr AS alias`. Kotlin `SqlAlias(expr, alias)` — the
    /// `alias` (always a `SqlIdentifier`) is flattened to its text.
    Alias { expr: Box<SqlExpr>, alias: String },
    /// `CAST(expr AS type)`. Kotlin `SqlCast(expr, dataType)` — `data_type`
    /// (always a `SqlIdentifier`) is flattened to its text.
    Cast { expr: Box<SqlExpr>, data_type: String },
    /// Sort key `expr ASC|DESC`. Kotlin `SqlSort(expr, asc)`.
    Sort { expr: Box<SqlExpr>, asc: bool },
    /// A SELECT statement (Kotlin's sole `SqlRelation`, `SqlSelect`). Boxed to
    /// break the size cycle `SqlExpr → SqlSelect → Option<SqlExpr>`: Kotlin
    /// hides this because every object is a heap reference, but a Rust value
    /// type needs explicit indirection to be sized.
    Select(Box<SqlSelect>),
}

/// A parsed `SELECT` statement. Kotlin: `data class SqlSelect(...)`.
#[derive(Debug, Clone, PartialEq)]
pub struct SqlSelect {
    pub projection: Vec<SqlExpr>,
    pub selection: Option<SqlExpr>,
    pub group_by: Vec<SqlExpr>,
    pub order_by: Vec<SqlExpr>,
    pub having: Option<SqlExpr>,
    pub limit: Option<i32>,
    pub table_name: String,
}
