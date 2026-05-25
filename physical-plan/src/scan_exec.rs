//! Port of `kquery/physical-plan/src/main/kotlin/ScanExec.kt`.
//!
//! Scans a data source with an optional push-down projection. It is the only leaf
//! operator: it has no child plan and produces its batches by delegating to the
//! `DataSource` (which the optimizer's `ProjectionPushDownRule` has already
//! trimmed to just the columns the query needs).

use crate::physical_plan::PhysicalPlan;
use datasource::DataSource;
use datatypes::{RecordBatch, Schema};
use std::fmt;
use std::sync::Arc;

/// Scan a data source with optional push-down projection.
/// Kotlin `ScanExec(val ds: DataSource, val projection: List<String>)`.
///
/// `ds` is held as `Arc<dyn DataSource>` (matching the logical `Scan` operator),
/// so the same source can be shared across plan nodes.
pub struct ScanExec {
    pub ds: Arc<dyn DataSource>,
    pub projection: Vec<String>,
}

impl ScanExec {
    pub fn new(ds: Arc<dyn DataSource>, projection: Vec<String>) -> Self {
        Self { ds, projection }
    }
}

impl PhysicalPlan for ScanExec {
    fn schema(&self) -> Schema {
        // Kotlin: `ds.schema().select(projection)`.
        self.ds.schema().select(&self.projection)
    }

    fn execute(&self) -> Box<dyn Iterator<Item = RecordBatch>> {
        // Kotlin: `ds.scan(projection)`.
        self.ds.scan(&self.projection)
    }

    fn children(&self) -> Vec<&dyn PhysicalPlan> {
        // A scan is a leaf — no inputs.
        vec![]
    }
}

impl fmt::Display for ScanExec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Kotlin: "ScanExec: schema=${schema()}, projection=$projection".
        // The datatypes `Schema` has no `Display`, so use its `Debug` form.
        write!(
            f,
            "ScanExec: schema={:?}, projection={:?}",
            self.schema(),
            self.projection
        )
    }
}
