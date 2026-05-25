//! Port of `kquery/physical-plan/src/main/kotlin/expressions/ColumnExpression.kt`.
//!
//! References a column in the input batch by its position. Evaluating it simply
//! hands back that column unchanged — the simplest possible physical expression.

use crate::expressions::Expression;
use datatypes::{record_batch, ColumnVector, RecordBatch};
use std::fmt;

/// Reference a column in a batch by index. Kotlin `ColumnExpression(val i: Int)`
/// (`Int` → `usize`, since it is an index).
pub struct ColumnExpression {
    pub i: usize,
}

impl ColumnExpression {
    pub fn new(i: usize) -> Self {
        Self { i }
    }
}

impl Expression for ColumnExpression {
    fn evaluate(&self, input: &RecordBatch) -> Box<dyn ColumnVector> {
        // Kotlin: `return input.field(i)`. `record_batch::field` wraps the
        // existing arrow `ArrayRef` (cheap, Arc-cloned) as a ColumnVector.
        Box::new(record_batch::field(input, self.i))
    }
}

impl fmt::Display for ColumnExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.i)
    }
}
