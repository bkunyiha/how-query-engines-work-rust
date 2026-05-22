//! Port of `kquery/logical-plan/src/main/kotlin/LogicalPlan.kt`.
//!
//! Kotlin declared `interface LogicalPlan { schema(); children() }` plus a
//! top-level `fun format(plan, indent)`. The interface's six implementors
//! (`Scan`, `Projection`, `Selection`, `Aggregate`, `Join`, `Limit`) collapse
//! into the `LogicalPlan` enum below; `schema` / `children` / `Display`
//! dispatch to the per-operator structs that live in their own files.

use crate::aggregate::Aggregate;
use crate::join::Join;
use crate::limit::Limit;
use crate::projection::Projection;
use crate::scan::Scan;
use crate::selection::Selection;
use datatypes::Schema;
use std::fmt;

/// A logical plan: a data transformation or action that returns a relation.
#[derive(Clone)]
pub enum LogicalPlan {
    Scan(Scan),
    Projection(Projection),
    Selection(Selection),
    Aggregate(Aggregate),
    Join(Join),
    Limit(Limit),
}

impl LogicalPlan {
    /// Schema of the data produced by this plan.
    pub fn schema(&self) -> Schema {
        match self {
            LogicalPlan::Scan(p) => p.schema(),
            LogicalPlan::Projection(p) => p.schema(),
            LogicalPlan::Selection(p) => p.schema(),
            LogicalPlan::Aggregate(p) => p.schema(),
            LogicalPlan::Join(p) => p.schema(),
            LogicalPlan::Limit(p) => p.schema(),
        }
    }

    /// Inputs of this plan (for walking the tree).
    pub fn children(&self) -> Vec<&LogicalPlan> {
        match self {
            LogicalPlan::Scan(p) => p.children(),
            LogicalPlan::Projection(p) => p.children(),
            LogicalPlan::Selection(p) => p.children(),
            LogicalPlan::Aggregate(p) => p.children(),
            LogicalPlan::Join(p) => p.children(),
            LogicalPlan::Limit(p) => p.children(),
        }
    }

    /// Human-readable, indented tree form. Kotlin `LogicalPlan.pretty()`.
    pub fn pretty(&self) -> String {
        format(self)
    }
}

impl fmt::Display for LogicalPlan {
    /// Single-line description of this node (Kotlin's per-class `toString`);
    /// the tree form is produced by [`format`].
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogicalPlan::Scan(p) => write!(f, "{p}"),
            LogicalPlan::Projection(p) => write!(f, "{p}"),
            LogicalPlan::Selection(p) => write!(f, "{p}"),
            LogicalPlan::Aggregate(p) => write!(f, "{p}"),
            LogicalPlan::Join(p) => write!(f, "{p}"),
            LogicalPlan::Limit(p) => write!(f, "{p}"),
        }
    }
}

/// Format a logical plan in human-readable form. Kotlin: `fun format(plan, indent = 0)`.
pub fn format(plan: &LogicalPlan) -> String {
    format_indent(plan, 0)
}

fn format_indent(plan: &LogicalPlan, indent: usize) -> String {
    let mut b = String::new();
    for _ in 0..indent {
        b.push('\t');
    }
    b.push_str(&plan.to_string());
    b.push('\n');
    for child in plan.children() {
        b.push_str(&format_indent(child, indent + 1));
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aggregate::Aggregate;
    use crate::expressions::{cast, col, lit_string, max};
    use crate::projection::Projection;
    use crate::scan::Scan;
    use crate::selection::Selection;
    use datasource::CsvDataSource;
    use datatypes::arrow_types::INT32_TYPE;
    use std::sync::Arc;

    fn employee_scan() -> Scan {
        let path = "../testdata/employee.csv";
        let csv = Arc::new(CsvDataSource::new(path, None, true, 10));
        Scan::new("employee", csv, vec![])
    }

    #[test]
    fn build_logical_plan_manually() {
        let scan = LogicalPlan::Scan(employee_scan());
        let selection =
            LogicalPlan::Selection(Selection::new(scan, col("state").eq(lit_string("CO"))));
        let plan = LogicalPlan::Projection(Projection::new(
            selection,
            vec![col("id"), col("first_name"), col("last_name")],
        ));

        assert_eq!(
            format(&plan),
            "Projection: #id, #first_name, #last_name\n\
             \tSelection: #state = 'CO'\n\
             \t\tScan: employee; projection=None\n"
        );
    }

    #[test]
    fn build_logical_plan_nested() {
        let plan = LogicalPlan::Projection(Projection::new(
            LogicalPlan::Selection(Selection::new(
                LogicalPlan::Scan(employee_scan()),
                col("state").eq(lit_string("CO")),
            )),
            vec![col("id"), col("first_name"), col("last_name")],
        ));

        assert_eq!(
            format(&plan),
            "Projection: #id, #first_name, #last_name\n\
             \tSelection: #state = 'CO'\n\
             \t\tScan: employee; projection=None\n"
        );
    }

    #[test]
    fn build_aggregate_plan() {
        let scan = LogicalPlan::Scan(employee_scan());
        let group_expr = vec![col("state")];
        let aggregate_expr = vec![max(cast(col("salary"), INT32_TYPE))];
        let plan = LogicalPlan::Aggregate(Aggregate::new(scan, group_expr, aggregate_expr));

        // NOTE: the upstream Kotlin asserts `Int(32, true)` because JVM Arrow's
        // `ArrowType.Int(32, true).toString()` renders that way. arrow-rs's
        // `DataType::Int32` renders as `Int32` via `Debug`, so the cast prints
        // `CAST(#salary AS Int32)`. Same semantics, different type-name string.
        assert_eq!(
            format(&plan),
            "Aggregate: groupExpr=[#state], aggregateExpr=[MAX(CAST(#salary AS Int32))]\n\
             \tScan: employee; projection=None\n"
        );
    }
}
