//! Port of `kquery/physical-plan/src/main/kotlin/LimitExec.kt`.
//!
//! Stops emitting rows once `limit` rows have been produced. Full batches pass
//! through untouched until the running budget would be exceeded; the boundary
//! batch is truncated to exactly the remaining count and the stream then ends.
//!
//! ## Translation note — `sequence { … yield … }` → `Iterator::scan`
//! Kotlin uses a coroutine builder (`sequence { var remaining = limit; for (batch in …) { … yield(…) } }`)
//! to carry the mutable `remaining` budget across batches. The Rust equivalent is
//! `Iterator::scan`, which threads a mutable state value through the stream and
//! ends iteration as soon as the closure returns `None` — exactly the early-exit
//! the Kotlin `break` provides.

use crate::physical_plan::PhysicalPlan;
use datatypes::{record_batch, ArrowVectorBuilder, ColumnVector, RecordBatch, Schema};

/// Execute a limit. Kotlin `LimitExec(val input: PhysicalPlan, val limit: Int)`
/// (`Int` → `usize`, a row count).
pub struct LimitExec {
    pub input: Box<dyn PhysicalPlan>,
    pub limit: usize,
}

impl LimitExec {
    pub fn new(input: Box<dyn PhysicalPlan>, limit: usize) -> Self {
        Self { input, limit }
    }
}

impl PhysicalPlan for LimitExec {
    fn schema(&self) -> Schema {
        self.input.schema()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn execute(&self) -> Box<dyn Iterator<Item = RecordBatch>> {
        let schema = self.input.schema();
        // `scan` carries `remaining` (the budget) across batches; returning `None`
        // ends the stream, mirroring the Kotlin coroutine's `break`.
        Box::new(self.input.execute().scan(self.limit, move |remaining, batch| {
            if *remaining == 0 {
                return None;
            }
            let rows = batch.num_rows();
            if rows <= *remaining {
                *remaining -= rows;
                Some(batch)
            } else {
                // Truncate this boundary batch to the remaining count, then stop.
                let take = *remaining;
                *remaining = 0;
                Some(truncate(&batch, take, &schema))
            }
        }))
    }

    fn children(&self) -> Vec<&dyn PhysicalPlan> {
        vec![self.input.as_ref()]
    }
}

impl std::fmt::Display for LimitExec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LimitExec: limit={}", self.limit)
    }
}

/// Build a new batch containing only the first `n` rows of `batch`, copying
/// cell-by-cell (Kotlin walks the source vector and `set`s into a fresh one).
fn truncate(batch: &RecordBatch, n: usize, schema: &Schema) -> RecordBatch {
    let columns: Vec<Box<dyn ColumnVector>> = (0..batch.num_columns())
        .map(|i| {
            let source = record_batch::field(batch, i);
            let mut builder = ArrowVectorBuilder::new(&source.get_type(), n);
            for row in 0..n {
                builder.append_value(&source.get_value(row));
            }
            builder.set_value_count(n);
            Box::new(builder.build()) as Box<dyn ColumnVector>
        })
        .collect();
    record_batch::create(schema, columns)
}

#[cfg(test)]
mod tests {
    //! Rust-port verification of the phase-2 end-to-end pipeline. There is no
    //! upstream operator test (the Kotlin operator tests live in `AggregateTest` /
    //! the SQL/query-planner suites), so this drives `ScanExec` →
    //! `Projection`/`Selection`/`LimitExec` over the `employee.csv` fixture and
    //! checks row/column counts. Uses `CsvDataSource` directly (the `fuzzer` crate
    //! is module 9, unported).
    use super::*;
    use crate::column_expression::ColumnExpression;
    use crate::expressions::LiteralLongExpression;
    use crate::boolean_expression::GtExpression;
    use crate::projection_exec::ProjectionExec;
    use crate::scan_exec::ScanExec;
    use crate::selection_exec::SelectionExec;
    use datasource::{CsvDataSource, DataSource};
    use std::sync::Arc;

    fn employee_ds() -> Arc<dyn DataSource> {
        Arc::new(CsvDataSource::new(
            "../testdata/employee.csv",
            None,
            true,
            1024,
        ))
    }

    /// All column names, in schema order: id, first_name, last_name, state,
    /// job_title, salary.
    fn all_columns(ds: &Arc<dyn DataSource>) -> Vec<String> {
        ds.schema().fields.iter().map(|f| f.name.clone()).collect()
    }

    fn total_rows(plan: &dyn PhysicalPlan) -> usize {
        plan.execute().map(|b| b.num_rows()).sum()
    }

    #[test]
    fn scan_reads_all_rows() {
        let ds = employee_ds();
        let scan = ScanExec::new(Arc::clone(&ds), all_columns(&ds));
        assert_eq!(total_rows(&scan), 4);
        // ScanExec is a leaf.
        assert!(scan.children().is_empty());
    }

    #[test]
    fn limit_truncates_to_budget() {
        let ds = employee_ds();
        let scan = ScanExec::new(Arc::clone(&ds), all_columns(&ds));
        let limited = LimitExec::new(Box::new(scan), 3);
        assert_eq!(total_rows(&limited), 3);
    }

    #[test]
    fn limit_above_total_keeps_everything() {
        let ds = employee_ds();
        let scan = ScanExec::new(Arc::clone(&ds), all_columns(&ds));
        let limited = LimitExec::new(Box::new(scan), 100);
        assert_eq!(total_rows(&limited), 4);
    }

    #[test]
    fn projection_keeps_one_column() {
        let ds = employee_ds();
        let scan = ScanExec::new(Arc::clone(&ds), all_columns(&ds));
        // Output schema is just the first column (id).
        let schema = scan.schema().project(&[0]);
        let proj = ProjectionExec::new(
            Box::new(scan),
            schema,
            vec![Arc::new(ColumnExpression::new(0))],
        );
        let batches: Vec<_> = proj.execute().collect();
        let rows: usize = batches.iter().map(|b| b.num_rows()).sum();
        assert_eq!(rows, 4);
        assert!(batches.iter().all(|b| b.num_columns() == 1));
    }

    #[test]
    fn selection_filters_rows() {
        let ds = employee_ds();
        let scan = ScanExec::new(Arc::clone(&ds), all_columns(&ds));
        // WHERE id > 2  →  ids 3 and 4  →  2 rows. `id` is a non-null Int64 column,
        // so the comparison never sees a null.
        let predicate = GtExpression::new(
            Arc::new(ColumnExpression::new(0)),
            Arc::new(LiteralLongExpression::new(2)),
        );
        let selection = SelectionExec::new(Box::new(scan), Arc::new(predicate));
        assert_eq!(total_rows(&selection), 2);
        // Selection preserves the schema (all six columns).
        assert_eq!(selection.schema().fields.len(), 6);
    }

    #[test]
    fn pipeline_scan_select_project_limit() {
        // End-to-end: scan → WHERE id > 2 → SELECT id → LIMIT 1.
        let ds = employee_ds();
        let scan = ScanExec::new(Arc::clone(&ds), all_columns(&ds));
        let selection = SelectionExec::new(
            Box::new(scan),
            Arc::new(GtExpression::new(
                Arc::new(ColumnExpression::new(0)),
                Arc::new(LiteralLongExpression::new(2)),
            )),
        );
        let project_schema = selection.schema().project(&[0]);
        let projection = ProjectionExec::new(
            Box::new(selection),
            project_schema,
            vec![Arc::new(ColumnExpression::new(0))],
        );
        let limited = LimitExec::new(Box::new(projection), 1);

        let batches: Vec<_> = limited.execute().collect();
        let rows: usize = batches.iter().map(|b| b.num_rows()).sum();
        assert_eq!(rows, 1);
        assert!(batches.iter().all(|b| b.num_columns() == 1));
    }
}
