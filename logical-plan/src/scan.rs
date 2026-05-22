//! Port of `kquery/logical-plan/src/main/kotlin/Scan.kt`.
//!
//! Represents a scan of a data source. The schema is derived once at
//! construction (Kotlin caches it in `val schema = deriveSchema()`).

use crate::logical_plan::LogicalPlan;
use datasource::DataSource;
use datatypes::Schema;
use std::fmt;
use std::sync::Arc;

/// A scan of a [`DataSource`], optionally projecting a subset of columns.
#[derive(Clone)]
pub struct Scan {
    pub path: String,
    pub data_source: Arc<dyn DataSource>,
    pub projection: Vec<String>,
    /// Cached derived schema (Kotlin `val schema = deriveSchema()`).
    schema: Schema,
}

impl Scan {
    pub fn new(
        path: impl Into<String>,
        data_source: Arc<dyn DataSource>,
        projection: Vec<String>,
    ) -> Self {
        let schema = Self::derive_schema(data_source.as_ref(), &projection);
        Self { path: path.into(), data_source, projection, schema }
    }

    /// Kotlin `deriveSchema()`: the full source schema, or the projected
    /// sub-schema when a projection is given.
    fn derive_schema(data_source: &dyn DataSource, projection: &[String]) -> Schema {
        let schema = data_source.schema();
        if projection.is_empty() {
            schema
        } else {
            schema.select(projection)
        }
    }

    pub fn schema(&self) -> Schema {
        self.schema.clone()
    }

    /// A scan has no inputs.
    pub fn children(&self) -> Vec<&LogicalPlan> {
        Vec::new()
    }
}

impl fmt::Display for Scan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.projection.is_empty() {
            write!(f, "Scan: {}; projection=None", self.path)
        } else {
            write!(f, "Scan: {}; projection=[{}]", self.path, self.projection.join(", "))
        }
    }
}
