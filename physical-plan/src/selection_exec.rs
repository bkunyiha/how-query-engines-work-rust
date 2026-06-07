//! Port of `kquery/physical-plan/src/main/kotlin/SelectionExec.kt`.
//!
//! Filters rows: evaluates a boolean predicate against each batch and keeps only
//! the rows where it is true. The schema is unchanged.
//!
//! ## Translation note — reading the selection vector
//! Kotlin downcasts the predicate result to a Java Arrow `BitVector`
//! (`(expr.evaluate(batch) as ArrowFieldVector).field as BitVector`) and reads
//! bits with `.get(i) == 1`. The Rust port stays at the `ColumnVector` abstraction
//! instead: the predicate evaluates to a boolean column, and each row is read as
//! `ScalarValue::Boolean(true)`. No downcast is needed, and it keeps the operator
//! working against any `ColumnVector` implementation.

use crate::expressions::Expression;
use crate::physical_plan::PhysicalPlan;
use datatypes::{record_batch, ArrowVectorBuilder, ColumnVector, RecordBatch, ScalarValue, Schema};
use std::sync::Arc;

/// Execute a selection (row filter). Kotlin
/// `SelectionExec(val input: PhysicalPlan, val expr: Expression)`.
pub struct SelectionExec {
    pub input: Arc<dyn PhysicalPlan>,
    pub expr: Arc<dyn Expression>,
}

impl SelectionExec {
    pub fn new(input: Arc<dyn PhysicalPlan>, expr: Arc<dyn Expression>) -> Self {
        Self { input, expr }
    }
}

impl PhysicalPlan for SelectionExec {
    fn schema(&self) -> Schema {
        self.input.schema()
    }

    fn execute(&self) -> Box<dyn Iterator<Item = RecordBatch>> {
        // Selection preserves the schema, so capture it once for all batches.
        let schema = self.input.schema();
        let expr = Arc::clone(&self.expr);
        Box::new(self.input.execute().map(move |batch| {
            let selection = expr.evaluate(&batch);
            let columns: Vec<Box<dyn ColumnVector>> = (0..batch.num_columns())
                .map(|i| filter(&record_batch::field(&batch, i), selection.as_ref()))
                .collect();
            record_batch::create(&schema, columns)
        }))
    }

    fn children(&self) -> Vec<&Arc<dyn PhysicalPlan>> {
        vec![&self.input]
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    /// Rebuild this selection with a new input child. See the trait-level
    /// `PhysicalPlan::with_new_children` doc for the general rewrite pattern.
    ///
    /// Arity 1: a selection has one input (the relation being filtered). The
    /// incoming `children` vec is therefore always length 1; `into_iter().next()
    /// .unwrap()` takes ownership of that single Arc without an atomic refcount
    /// bump. The predicate `expr` is reused — it doesn't depend on which
    /// concrete input feeds the selection.
    fn with_new_children(
        self: Arc<Self>,
        children: Vec<Arc<dyn PhysicalPlan>>,
    ) -> Arc<dyn PhysicalPlan> {
        assert_eq!(
            children.len(),
            1,
            "SelectionExec expects exactly 1 child"
        );
        Arc::new(SelectionExec::new(
            children.into_iter().next().unwrap(),
            Arc::clone(&self.expr),
        ))
    }
}

impl std::fmt::Display for SelectionExec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Kotlin's SelectionExec has no explicit toString; give it a readable one.
        write!(f, "SelectionExec: {}", self.expr)
    }
}

/// Keep the cells of `v` whose corresponding row in the boolean `selection`
/// column is true, returning a new (shorter) column of the same type.
/// Kotlin's private `filter(v, selection: BitVector)`.
fn filter(v: &dyn ColumnVector, selection: &dyn ColumnVector) -> Box<dyn ColumnVector> {
    // Count selected rows first, to size the builder (Kotlin does the same).
    let mut count = 0usize;
    for i in 0..selection.size() {
        if matches!(selection.get_value(i), ScalarValue::Boolean(true)) {
            count += 1;
        }
    }

    let mut builder = ArrowVectorBuilder::new(&v.get_type(), count);
    for i in 0..selection.size() {
        if matches!(selection.get_value(i), ScalarValue::Boolean(true)) {
            builder.append_value(&v.get_value(i));
        }
    }
    builder.set_value_count(count);
    Box::new(builder.build())
}
