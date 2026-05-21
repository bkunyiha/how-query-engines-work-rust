//! Port of `kquery/datasource/src/main/kotlin/CsvDataSource.kt`.
//!
//! CSV data source. This is one of the explicit
//! library-forced substitutions: instead of porting Andy's ~269 lines of hand-rolled
//! univocity-parsers logic (schema inference, batch-by-batch reading, per-type
//! field-vector setters), the Rust port delegates to `arrow::csv::ReaderBuilder`,
//! which does all of those things natively and produces `RecordBatch`es directly.
//!
//! Translation notes:
//! - Kotlin `CsvDataSource(filename, schema, hasHeaders, batchSize)` → Rust
//!   `CsvDataSource { filename, schema, has_headers, batch_size, delimiter }`.
//!   Added `delimiter` so the same struct can serve TSV (`\t`) — kquery had
//!   separate test paths for `.tsv` but used the same class with univocity's
//!   `isDelimiterDetectionEnabled`. arrow-rs requires explicit delimiter.
//! - Kotlin `Sequence<RecordBatch>` → `Box<dyn Iterator<Item = RecordBatch>>`.
//! - Kotlin schema inference (everything-as-String) → arrow-rs's typed inference
//!   (picks numeric / bool / utf8 by scanning rows). This is a *behaviour*
//!   change, but it matches the better default and produces more useful schemas.
//! - Kotlin `Schema.select(projection)` for the projected schema → same in Rust.
//!   arrow-rs's `with_projection` takes column *indices*, so we resolve names
//!   to indices here before passing them in.
//! - Errors panic (`File::open` failure, malformed CSV, etc.).

use crate::data_source::DataSource;
use arrow::csv::{reader::Format, ReaderBuilder};
use datatypes::{schema::from_arrow as schema_from_arrow, RecordBatch, Schema};
use std::fs::File;
use std::sync::Arc;

pub struct CsvDataSource {
    pub filename:    String,
    /// If `None`, the schema is inferred from the file on first access.
    pub schema:      Option<Schema>,
    pub has_headers: bool,
    pub batch_size:  usize,
    pub delimiter:   u8,
}

impl CsvDataSource {
    /// Construct a CSV source. `delimiter` is typically `b','` (the default if
    /// you use [`CsvDataSource::new`]). Use [`CsvDataSource::tsv`] for TSV.
    pub fn new(
        filename: impl Into<String>,
        schema: Option<Schema>,
        has_headers: bool,
        batch_size: usize,
    ) -> Self {
        Self {
            filename: filename.into(),
            schema,
            has_headers,
            batch_size,
            delimiter: b',',
        }
    }

    /// Convenience constructor for tab-separated files.
    pub fn tsv(
        filename: impl Into<String>,
        schema: Option<Schema>,
        has_headers: bool,
        batch_size: usize,
    ) -> Self {
        let mut s = Self::new(filename, schema, has_headers, batch_size);
        s.delimiter = b'\t';
        s
    }

    /// Infer the schema by scanning the file. Kotlin's `inferSchema()` infers
    /// everything as `String`; we use arrow-rs's typed inference (`Int64`,
    /// `Float64`, `Boolean`, `Utf8`) which is more useful.
    fn infer_schema(&self) -> Schema {
        let file = File::open(&self.filename).unwrap_or_else(|e| {
            panic!("CsvDataSource::infer_schema: cannot open '{}': {}", self.filename, e)
        });
        let format = Format::default()
            .with_header(self.has_headers)
            .with_delimiter(self.delimiter);
        let (arrow_schema, _records_read) = format
            .infer_schema(&file, Some(1024))
            .unwrap_or_else(|e| panic!("CsvDataSource::infer_schema: {}", e));
        schema_from_arrow(&arrow_schema)
    }
}

impl DataSource for CsvDataSource {
    fn schema(&self) -> Schema {
        self.schema.clone().unwrap_or_else(|| self.infer_schema())
    }

    fn scan(&self, projection: &[String]) -> Box<dyn Iterator<Item = RecordBatch>> {
        let file = File::open(&self.filename).unwrap_or_else(|e| {
            panic!("CsvDataSource::scan: cannot open '{}': {}", self.filename, e)
        });

        // Determine the schema used by the reader (typed schema, not projected).
        let full_schema = self.schema();
        let full_arrow_schema = Arc::new(full_schema.to_arrow());

        // Build the reader. Note: `with_projection` requires column indices.
        let mut builder = ReaderBuilder::new(full_arrow_schema.clone())
            .with_header(self.has_headers)
            .with_batch_size(self.batch_size)
            .with_delimiter(self.delimiter);

        if !projection.is_empty() {
            // Resolve names to indices in the FULL schema.
            let indices: Vec<usize> = projection
                .iter()
                .map(|name| {
                    full_schema
                        .fields
                        .iter()
                        .position(|f| &f.name == name)
                        .unwrap_or_else(|| {
                            panic!(
                                "CsvDataSource::scan: projection column '{}' not in schema",
                                name
                            )
                        })
                })
                .collect();
            builder = builder.with_projection(indices);
        }

        let reader = builder.build(file).unwrap_or_else(|e| {
            panic!("CsvDataSource::scan: failed to build CSV reader: {}", e)
        });

        // The reader is itself an Iterator<Item = Result<RecordBatch, ArrowError>>.
        // Unwrap and panic on parse errors rather than propagating Result.
        Box::new(reader.map(|res| {
            res.unwrap_or_else(|e| panic!("CsvDataSource: malformed CSV batch: {}", e))
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use datatypes::record_batch::row_count;

    // Test data fixtures live at testdata/employee.csv etc., relative to the
    // workspace root. Cargo runs tests from the crate directory, so we point
    // back up one level.
    fn fixture(name: &str) -> String {
        format!("../testdata/{}", name)
    }

    #[test]
    fn read_csv_with_no_projection() {
        let csv = CsvDataSource::new(fixture("employee.csv"), None, true, 1024);
        let batches: Vec<_> = csv.scan(&[]).collect();
        assert_eq!(batches.len(), 1);
        let b = &batches[0];
        // employee.csv has 4 rows (per kquery CsvDataSourceTest).
        assert_eq!(row_count(b), 4);
        // 6 columns: id, first_name, last_name, state, job_title, salary.
        assert_eq!(b.num_columns(), 6);
        // Bind the schema to a local so the &str borrows from f.name() outlive
        // the statement (the SchemaRef returned by b.schema() is otherwise a
        // temporary that drops at the semicolon).
        let schema = b.schema();
        let names: Vec<&str> = schema.fields().iter().map(|f| f.name().as_str()).collect();
        for expected in ["id", "first_name", "last_name", "state", "job_title", "salary"] {
            assert!(names.contains(&expected), "missing column: {}", expected);
        }
    }

    #[test]
    fn read_csv_with_projection() {
        let csv = CsvDataSource::new(fixture("employee.csv"), None, true, 1024);
        let projection = vec![
            "first_name".to_string(),
            "last_name".to_string(),
            "state".to_string(),
        ];
        let batches: Vec<_> = csv.scan(&projection).collect();
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].num_columns(), 3);
        assert_eq!(row_count(&batches[0]), 4);
    }

    #[test]
    fn read_csv_with_small_batch_splits_into_multiple_batches() {
        let csv = CsvDataSource::new(fixture("employee.csv"), None, true, 1);
        let batches: Vec<_> = csv.scan(&[]).collect();
        // 4 rows, batch size 1 → 4 batches.
        assert_eq!(batches.len(), 4);
        for b in &batches {
            assert_eq!(row_count(b), 1);
        }
    }

    /// Note on the TSV test fixtures: `testdata/employee.tsv` is *not* actually
    /// tab-separated despite its extension — it uses two-space whitespace
    /// alignment between columns. Kotlin's tests pass against it only because
    /// univocity-parsers auto-detects the delimiter via heuristics
    /// (`isDelimiterDetectionEnabled = true`). arrow-rs's CSV reader needs an
    /// explicit delimiter and does not support multi-space "delimiters", so the
    /// Rust port uses `testdata/employee_no_header.tsv` (which IS actually
    /// tab-separated, hex `0x09`) for the TSV smoke test.
    #[test]
    fn read_tsv_no_header() {
        // employee_no_header.tsv is real tab-separated, no header row.
        // Provide an explicit schema since there's no header to infer names from.
        use datatypes::arrow_types::STRING_TYPE;
        use datatypes::{Field, Schema};
        let schema = Schema::new(vec![
            Field::new("field_1", STRING_TYPE),
            Field::new("field_2", STRING_TYPE),
            Field::new("field_3", STRING_TYPE),
            Field::new("field_4", STRING_TYPE),
            Field::new("field_5", STRING_TYPE),
            Field::new("field_6", STRING_TYPE),
        ]);
        let csv = CsvDataSource::tsv(
            fixture("employee_no_header.tsv"),
            Some(schema),
            false,
            1024,
        );
        let batches: Vec<_> = csv.scan(&[]).collect();
        assert_eq!(batches.len(), 1);
        // employee_no_header.tsv has 3 rows.
        assert_eq!(row_count(&batches[0]), 3);
        // 6 columns, all parsed as strings since the schema was forced to all-Utf8.
        assert_eq!(batches[0].num_columns(), 6);
    }
}
