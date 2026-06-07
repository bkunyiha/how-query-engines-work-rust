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
    pub input: Arc<dyn PhysicalPlan>,
    pub schema: Schema,
    pub expr: Vec<Arc<dyn Expression>>,
}

impl ProjectionExec {
    pub fn new(input: Arc<dyn PhysicalPlan>, schema: Schema, expr: Vec<Arc<dyn Expression>>) -> Self {
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

    fn children(&self) -> Vec<&Arc<dyn PhysicalPlan>> {
        vec![&self.input]
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    /// Rebuild this projection with a new input child.
    ///
    /// Plan-tree rewrites (e.g. `DistributedPlanner::substitute_shuffle_reader`)
    /// walk the tree generically: at each node they recurse into `children()`,
    /// transform any leaves they care about, then call `with_new_children` to
    /// reassemble the node with the rewritten inputs. The node keeps its own
    /// expressions/schema — only the inputs swap.
    ///
    /// `ProjectionExec` has arity 1 (one input relation), so the incoming
    /// `children` vec always has exactly one element. We:
    ///
    /// 1. Assert the arity invariant (catches planner bugs early).
    /// 2. Consume the vec via `into_iter().next().unwrap()` to take ownership
    ///    of that single `Arc<dyn PhysicalPlan>` without an atomic refcount
    ///    bump. (DataFusion equivalently writes `children[0].clone()`, which
    ///    bumps the refcount instead — the difference is negligible.)
    /// 3. Reuse `self.schema` and `self.expr` — they don't depend on which
    ///    concrete input feeds this projection, only on the projection's own
    ///    definition.
    fn with_new_children(
        self: Arc<Self>,
        children: Vec<Arc<dyn PhysicalPlan>>,
    ) -> Arc<dyn PhysicalPlan> {
        assert_eq!(
            children.len(),
            1,
            "ProjectionExec expects exactly 1 child"
        );
        Arc::new(ProjectionExec::new(
            children.into_iter().next().unwrap(),
            self.schema.clone(),
            self.expr.clone(),
        ))
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
