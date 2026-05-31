//! Port of `kquery/physical-plan/src/main/kotlin/ProjectionExec.kt`.
//!
//! Evaluates a list of expressions against each input batch and assembles the
//! results into an output batch with the projection's schema.

use crate::expressions::Expression;
use crate::physical_plan::PhysicalPlan;
use datatypes::{record_batch, ColumnVector, RecordBatch, Schema};
use std::fmt;
use std::sync::Arc;

/// Execute a projection. Kotlin
/// `ProjectionExec(val input: PhysicalPlan, val schema: Schema, val expr: List<Expression>)`.
///
/// The output schema is supplied explicitly (the query planner computes it) — a
/// projection can rename or compute columns, so it cannot always be derived from
/// the input.
pub struct ProjectionExec {
    pub input: Box<dyn PhysicalPlan>,
    pub schema: Schema,
    pub expr: Vec<Arc<dyn Expression>>,
}

impl ProjectionExec {
    pub fn new(input: Box<dyn PhysicalPlan>, schema: Schema, expr: Vec<Arc<dyn Expression>>) -> Self {
        Self {
            input,
            schema,
            expr,
        }
    }
}

impl PhysicalPlan for ProjectionExec {
    fn schema(&self) -> Schema {
        self.schema.clone()
    }

    fn execute(&self) -> Box<dyn Iterator<Item = RecordBatch>> {
        // Kotlin: `input.execute().map { batch -> RecordBatch(schema, expr.map { it.evaluate(batch) }) }`.
        // We clone the schema and the (Arc-wrapped) expressions into the closure so
        // the returned iterator owns everything it needs (it must be `'static`).
        let schema = self.schema.clone();
        let exprs = self.expr.clone();
        Box::new(self.input.execute().map(move |batch| {
            let columns: Vec<Box<dyn ColumnVector>> =
                exprs.iter().map(|e| e.evaluate(&batch)).collect();
            record_batch::create(&schema, columns)
        }))
    }

    fn children(&self) -> Vec<&dyn PhysicalPlan> {
        vec![self.input.as_ref()]
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl fmt::Display for ProjectionExec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Kotlin: "ProjectionExec: $expr", where `expr` is a List whose toString is
        // "[a, b, c]". Mirror that bracketed, comma-separated form.
        let exprs: Vec<String> = self.expr.iter().map(|e| e.to_string()).collect();
        write!(f, "ProjectionExec: [{}]", exprs.join(", "))
    }
}
