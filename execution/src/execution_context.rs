//! Port of `kquery/execution/src/main/kotlin/ExecutionContext.kt`.
//!
//! The single-node front door to the engine. `ExecutionContext` ties the whole
//! pipeline together: it parses SQL (or accepts a `DataFrame` built fluently),
//! optimizes the logical plan, lowers it to a physical plan, and executes it,
//! yielding a stream of [`RecordBatch`]es. Most user-facing code and tests call
//! through here.
//!
//! ## Translation notes
//! - **`Map<String, String>` → `HashMap<String, String>`** for settings, and the
//!   `rquery.csv.batchSize` setting is read the same way (default 1024).
//! - **`Sequence<RecordBatch>` → `Box<dyn Iterator<Item = RecordBatch>>`** — the
//!   same lazy-stream shape used by `PhysicalPlan::execute` (see the
//!   `physical_plan` module note).
//! - **Overloads → distinct names.** Kotlin overloads `execute(df)` and
//!   `execute(plan)`. Rust can't overload by argument type, so the plan-taking
//!   method keeps the name [`ExecutionContext::execute`] and the DataFrame-taking
//!   one becomes [`ExecutionContext::execute_data_frame`].
//! - **`register*` take `&mut self`.** Kotlin mutates a `mutableMapOf`; the
//!   idiomatic Rust equivalent is `&mut self` plus a plain `HashMap`, which also
//!   keeps the context `Send + Sync` (no interior mutability) so `ParallelContext`
//!   can share it with rayon workers.
//! - **`sql()` parses with the hand-ported Pratt parser** (`SqlParser::parse(0)`),
//!   then expects a `SqlExpr::Select`; anything else panics (§3.6), matching
//!   Kotlin's `as SqlSelect` cast. The vestigial `DataFrameImpl(df.logicalPlan())`
//!   re-wrap is dropped — `create_data_frame` already returns the `DataFrame`.

use std::collections::HashMap;
use std::sync::Arc;

use datasource::{CsvDataSource, DataSource};
use datatypes::RecordBatch;
use logical_plan::{DataFrame, LogicalPlan, Scan};
use optimizer::Optimizer;
use physical_plan::ExecutorContext;
use query_planner::QueryPlanner;
// `PrattParser` brings the `parse` method into scope for `SqlParser`.
use sql::{PrattParser, SqlExpr, SqlParser, SqlPlanner, SqlTokenizer};

/// Default CSV batch size when `rquery.csv.batchSize` is unset. Kotlin: `"1024"`.
const DEFAULT_BATCH_SIZE: usize = 1024;

/// Single-node execution context. Kotlin `class ExecutionContext`.
pub struct ExecutionContext {
    /// Configuration settings. Kotlin `val settings: Map<String, String>`.
    pub settings: HashMap<String, String>,
    /// CSV read batch size, derived from `settings` once at construction.
    batch_size: usize,
    /// Tables registered with this context (Kotlin `private val tables`).
    tables: HashMap<String, DataFrame>,
}

impl ExecutionContext {
    /// Kotlin `ExecutionContext(settings)`.
    pub fn new(settings: HashMap<String, String>) -> Self {
        let batch_size = settings
            .get("rquery.csv.batchSize")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(DEFAULT_BATCH_SIZE);
        Self { settings, batch_size, tables: HashMap::new() }
    }

    /// The configured CSV batch size. Kotlin `val batchSize`.
    pub fn batch_size(&self) -> usize {
        self.batch_size
    }

    /// Create a `DataFrame` for the given SQL `SELECT`. Kotlin `sql(sql)`.
    pub fn sql(&self, sql: &str) -> DataFrame {
        let tokens = SqlTokenizer::new(sql).tokenize();
        let parsed = SqlParser::new(tokens).parse(0);
        let select = match parsed {
            Some(SqlExpr::Select(select)) => *select,
            other => panic!("Expected a SELECT statement, found {other:?}"),
        };
        SqlPlanner::new().create_data_frame(&select, &self.tables)
    }

    /// Get a `DataFrame` representing the specified CSV file. Kotlin `csv(filename)`.
    pub fn csv(&self, filename: &str) -> DataFrame {
        let source = CsvDataSource::new(filename, None, true, self.batch_size);
        DataFrame::new(LogicalPlan::Scan(Scan::new(filename, Arc::new(source), vec![])))
    }

    /// Register a `DataFrame` with the context. Kotlin `register(tablename, df)`.
    pub fn register(&mut self, table_name: &str, df: DataFrame) {
        self.tables.insert(table_name.to_string(), df);
    }

    /// Register a data source with the context. Kotlin `registerDataSource`.
    pub fn register_data_source(&mut self, table_name: &str, data_source: Arc<dyn DataSource>) {
        let scan = Scan::new(table_name, data_source, vec![]);
        self.register(table_name, DataFrame::new(LogicalPlan::Scan(scan)));
    }

    /// Register a CSV data source with the context. Kotlin `registerCsv`.
    pub fn register_csv(&mut self, table_name: &str, filename: &str) {
        let df = self.csv(filename);
        self.register(table_name, df);
    }

    /// Execute the logical plan represented by a `DataFrame`. Kotlin `execute(df)`.
    pub fn execute_data_frame(&self, df: &DataFrame) -> Box<dyn Iterator<Item = RecordBatch>> {
        self.execute(df.logical_plan())
    }

    /// Execute the provided logical plan. Kotlin `execute(plan)`: optimize, lower
    /// to a physical plan, and run it.
    ///
    /// Constructs a single-process `ExecutorContext` to satisfy the trait
    /// signature. Non-shuffle operators ignore it; the executor identity is
    /// `"single-node"` and the shuffle directory is a default path that's
    /// never actually written to (no shuffle ops run in single-process mode).
    pub fn execute(&self, plan: &LogicalPlan) -> Box<dyn Iterator<Item = RecordBatch>> {
        let optimized = Optimizer::new().optimize(plan);
        let physical = QueryPlanner::new().create_physical_plan(&optimized);
        let ctx = ExecutorContext::new("single-node", "localhost", 0, "/tmp/rquery-single-node");
        physical.execute(&ctx)
    }
}

#[cfg(test)]
mod tests {
    //! Port of `ExecutionSqlTest.kt` (logical-plan-string assertions) and the
    //! `ExecutionTest.kt` cases. The `Fuzzer`-backed cases — `min max sum float`,
    //! `float math`, `boolean expressions`, `inner join using DataFrame`,
    //! `left join using DataFrame` — use the now-ported `fuzzer` crate (module 9).
    //! The `date and interval arithmetic` case is no longer blocked at the
    //! planner (`LiteralDate` now lowers via `chrono::NaiveDate` → days-since-
    //! Unix-epoch, matching Kotlin's `LocalDate.toEpochDay()`); a port of that
    //! test can land alongside any remaining `DateSubtractInterval` work.
    //!
    //! ## Float formatting note
    //! Kotlin's `Float.toString()` emits `"1.0"` for whole-valued floats; Rust's
    //! `f32::to_string()` (which `datatypes::record_batch::to_csv` uses) emits
    //! `"1"`. The Fuzzer-backed float tests below assert against Rust's actual
    //! output, so `min max sum float` checks `"a,1,2,3"` rather than Kotlin's
    //! `"a,1.0,2.0,3.0"`; `float_math` computes its expected division literally
    //! (`let q = 1.0_f32 / 11.0_f32`) so the assertion matches whatever Rust's
    //! formatter produces. Logged in `TRANSLATION_NOTES.md` under module
    //! `execution` / `datatypes`.
    use super::*;
    use datasource::InMemoryDataSource;
    use datatypes::arrow_types::{BOOLEAN_TYPE, FLOAT_TYPE, INT32_TYPE, STRING_TYPE};
    use datatypes::record_batch::to_csv;
    use datatypes::{Field, ScalarValue, Schema};
    use fuzzer::Fuzzer;
    use logical_plan::{cast, col, format, lit_string, max, min, sum, JoinType};
    use std::collections::HashSet;

    /// Helper: wrap a single in-memory `RecordBatch` as a `DataFrame` over a
    /// scan of an `InMemoryDataSource`. Mirrors the
    /// `DataFrameImpl(Scan("", InMemoryDataSource(schema, listOf(batch)), listOf()))`
    /// shape repeated in every Kotlin `ExecutionTest` Fuzzer case.
    fn in_memory_df(name: &str, schema: Schema, batch: RecordBatch) -> DataFrame {
        let source = InMemoryDataSource::new(schema, vec![batch]);
        DataFrame::new(LogicalPlan::Scan(Scan::new(name, Arc::new(source), vec![])))
    }

    const EMPLOYEE_CSV: &str = "../testdata/employee.csv";

    fn ctx_with_employee() -> ExecutionContext {
        let mut ctx = ExecutionContext::new(HashMap::new());
        ctx.register_csv("employee", EMPLOYEE_CSV);
        ctx
    }

    // ---- ExecutionSqlTest: ctx.sql() builds the expected logical plan ----

    #[test]
    fn simple_select() {
        let ctx = ctx_with_employee();
        let df = ctx.sql("SELECT id FROM employee");
        assert_eq!(
            format(df.logical_plan()),
            "Projection: #id\n\tScan: ../testdata/employee.csv; projection=None\n"
        );
    }

    #[test]
    fn select_with_where() {
        let ctx = ctx_with_employee();
        let df = ctx.sql("SELECT id FROM employee WHERE state = 'CO'");
        assert_eq!(
            format(df.logical_plan()),
            "Projection: #id\n\
             \tSelection: #state = 'CO'\n\
             \t\tProjection: #id, #state\n\
             \t\t\tScan: ../testdata/employee.csv; projection=None\n"
        );
    }

    #[test]
    fn select_with_aliased_binary_expression() {
        let ctx = ctx_with_employee();
        let df = ctx.sql("SELECT salary * 0.1 AS bonus FROM employee");
        assert_eq!(
            format(df.logical_plan()),
            "Projection: #salary * 0.1 as bonus\n\
             \tScan: ../testdata/employee.csv; projection=None\n"
        );
    }

    #[test]
    fn selection_referencing_aliased_expression() {
        let ctx = ctx_with_employee();
        let df = ctx.sql(
            "SELECT salary AS annual_salary FROM employee \
             WHERE annual_salary > 1000 AND state = 'CO'",
        );
        assert_eq!(
            format(df.logical_plan()),
            "Projection: #annual_salary\n\
             \tSelection: #annual_salary > 1000 AND #state = 'CO'\n\
             \t\tProjection: #salary as annual_salary, #state\n\
             \t\t\tScan: ../testdata/employee.csv; projection=None\n"
        );
    }

    // ---- ExecutionTest: end-to-end execute() over employee.csv ----

    #[test]
    fn employees_in_co_using_dataframe() {
        let ctx = ExecutionContext::new(HashMap::new());
        let df = ctx
            .csv(EMPLOYEE_CSV)
            .filter(col("state").eq(lit_string("CO")))
            .project(vec![col("id"), col("first_name"), col("last_name")]);
        let batches: Vec<RecordBatch> = ctx.execute_data_frame(&df).collect();
        assert_eq!(batches.len(), 1);
        assert_eq!(to_csv(&batches[0]), "2,Gregg,Langford\n3,John,Travis\n");
    }

    #[test]
    fn employees_in_ca_using_sql() {
        let mut ctx = ExecutionContext::new(HashMap::new());
        ctx.register_csv("employee", EMPLOYEE_CSV);
        let df = ctx.sql("SELECT id, first_name, last_name FROM employee WHERE state = 'CA'");
        let batches: Vec<RecordBatch> = ctx.execute_data_frame(&df).collect();
        assert_eq!(batches.len(), 1);
        assert_eq!(to_csv(&batches[0]), "1,Bill,Hopkins\n");
    }

    #[test]
    fn aggregate_query() {
        // SELECT state, MAX(CAST(salary AS int)) ... GROUP BY state. The output
        // row order is HashMap-driven (non-deterministic), so assert the set of
        // rows rather than their order. Rust's `to_csv` renders the null-state
        // group as "null,11500"; we check the two named groups + the row count.
        let ctx = ExecutionContext::new(HashMap::new());
        let df = ctx
            .csv(EMPLOYEE_CSV)
            .aggregate(vec![col("state")], vec![max(cast(col("salary"), INT32_TYPE))]);
        let batches: Vec<RecordBatch> = ctx.execute_data_frame(&df).collect();
        assert_eq!(batches.len(), 1);

        let rows: HashSet<String> = to_csv(&batches[0]).lines().map(str::to_string).collect();
        assert_eq!(rows.len(), 3, "expected three group rows, got {rows:?}");
        assert!(rows.contains("CA,12000"), "missing CA group in {rows:?}");
        assert!(rows.contains("CO,11500"), "missing CO group in {rows:?}");
    }

    #[test]
    fn limit_using_dataframe() {
        let ctx = ExecutionContext::new(HashMap::new());
        let df = ctx
            .csv(EMPLOYEE_CSV)
            .project(vec![col("id"), col("first_name"), col("last_name")])
            .limit(2);
        let batches: Vec<RecordBatch> = ctx.execute_data_frame(&df).collect();
        assert_eq!(batches.len(), 1);
        assert_eq!(to_csv(&batches[0]), "1,Bill,Hopkins\n2,Gregg,Langford\n");
    }

    #[test]
    fn limit_using_sql() {
        let mut ctx = ExecutionContext::new(HashMap::new());
        ctx.register_csv("employee", EMPLOYEE_CSV);
        let df = ctx.sql("SELECT id, first_name, last_name FROM employee LIMIT 2");
        let batches: Vec<RecordBatch> = ctx.execute_data_frame(&df).collect();
        assert_eq!(batches.len(), 1);
        assert_eq!(to_csv(&batches[0]), "1,Bill,Hopkins\n2,Gregg,Langford\n");
    }

    #[test]
    fn limit_with_filter_using_sql() {
        let mut ctx = ExecutionContext::new(HashMap::new());
        ctx.register_csv("employee", EMPLOYEE_CSV);
        let df =
            ctx.sql("SELECT id, first_name, last_name FROM employee WHERE state = 'CO' LIMIT 1");
        let batches: Vec<RecordBatch> = ctx.execute_data_frame(&df).collect();
        assert_eq!(batches.len(), 1);
        assert_eq!(to_csv(&batches[0]), "2,Gregg,Langford\n");
    }

    // ---- ExecutionTest: Fuzzer-backed cases (unblocked by module 9) ----

    #[test]
    fn min_max_sum_float() {
        // Kotlin asserts the exact ordered CSV "a,1.0,2.0,3.0\nb,3.0,4.0,7.0\n".
        // The HashMap-driven aggregate has non-deterministic output order, so we
        // assert as a row SET (same approach as `aggregate_query` above). Float
        // formatting also differs — see the module-level float-formatting note.
        let schema = Schema::new(vec![
            Field::new("a", STRING_TYPE),
            Field::new("b", FLOAT_TYPE),
        ]);
        let batch = Fuzzer::new().create_record_batch(
            &schema,
            vec![
                vec![
                    ScalarValue::Utf8("a".into()),
                    ScalarValue::Utf8("a".into()),
                    ScalarValue::Utf8("b".into()),
                    ScalarValue::Utf8("b".into()),
                ],
                vec![
                    ScalarValue::Float32(1.0),
                    ScalarValue::Float32(2.0),
                    ScalarValue::Float32(4.0),
                    ScalarValue::Float32(3.0),
                ],
            ],
        );

        let ctx = ExecutionContext::new(HashMap::new());
        let df = in_memory_df("test", schema, batch).aggregate(
            vec![col("a")],
            vec![min(col("b")), max(col("b")), sum(col("b"))],
        );
        let batches: Vec<RecordBatch> = ctx.execute_data_frame(&df).collect();
        assert_eq!(batches.len(), 1);

        let rows: HashSet<String> = to_csv(&batches[0]).lines().map(str::to_string).collect();
        assert_eq!(rows.len(), 2, "expected two group rows, got {rows:?}");
        assert!(rows.contains("a,1,2,3"), "missing 'a' group in {rows:?}");
        assert!(rows.contains("b,3,4,7"), "missing 'b' group in {rows:?}");
    }

    #[test]
    fn float_math() {
        // Project a/b/a*b/a/b over four (a,b) pairs where a/b is always 1/11.
        // Compute `q = 1.0_f32 / 11.0_f32` literally so the expected string
        // matches whatever Rust's f32 formatter produces — no guesswork about
        // float precision. (Kotlin's expected string was "0.09090909" formatted
        // by the JVM; Rust may format the same f32 value identically or with a
        // different number of significant digits, and either way the equality
        // holds because we use the same `1.0_f32 / 11.0_f32` value.)
        let schema = Schema::new(vec![
            Field::new("a", FLOAT_TYPE),
            Field::new("b", FLOAT_TYPE),
        ]);
        let batch = Fuzzer::new().create_record_batch(
            &schema,
            vec![
                vec![
                    ScalarValue::Float32(1.0),
                    ScalarValue::Float32(2.0),
                    ScalarValue::Float32(4.0),
                    ScalarValue::Float32(3.0),
                ],
                vec![
                    ScalarValue::Float32(11.0),
                    ScalarValue::Float32(22.0),
                    ScalarValue::Float32(44.0),
                    ScalarValue::Float32(33.0),
                ],
            ],
        );

        let ctx = ExecutionContext::new(HashMap::new());
        let df = in_memory_df("test", schema, batch).project(vec![
            col("a").add(col("b")),
            col("a").subtract(col("b")),
            col("a").mult(col("b")),
            col("a").div(col("b")),
        ]);
        let batches: Vec<RecordBatch> = ctx.execute_data_frame(&df).collect();
        assert_eq!(batches.len(), 1);

        // a/b is 1/11 for every row by construction.
        let q = 1.0_f32 / 11.0_f32;
        let expected = format!(
            "12,-10,11,{q}\n24,-20,44,{q}\n48,-40,176,{q}\n36,-30,99,{q}\n"
        );
        assert_eq!(to_csv(&batches[0]), expected);
    }

    #[test]
    fn boolean_expressions() {
        let schema = Schema::new(vec![
            Field::new("a", BOOLEAN_TYPE),
            Field::new("b", BOOLEAN_TYPE),
        ]);
        let batch = Fuzzer::new().create_record_batch(
            &schema,
            vec![
                vec![
                    ScalarValue::Boolean(false),
                    ScalarValue::Boolean(false),
                    ScalarValue::Boolean(true),
                    ScalarValue::Boolean(true),
                ],
                vec![
                    ScalarValue::Boolean(false),
                    ScalarValue::Boolean(true),
                    ScalarValue::Boolean(false),
                    ScalarValue::Boolean(true),
                ],
            ],
        );

        let ctx = ExecutionContext::new(HashMap::new());
        let df = in_memory_df("test", schema, batch).project(vec![
            col("a").and(col("b")),
            col("a").or(col("b")),
        ]);
        let batches: Vec<RecordBatch> = ctx.execute_data_frame(&df).collect();
        assert_eq!(batches.len(), 1);
        assert_eq!(
            to_csv(&batches[0]),
            "false,false\nfalse,true\nfalse,true\ntrue,true\n",
        );
    }

    #[test]
    fn inner_join_using_dataframe() {
        let left_schema = Schema::new(vec![
            Field::new("id", INT32_TYPE),
            Field::new("name", STRING_TYPE),
        ]);
        let right_schema = Schema::new(vec![
            Field::new("id", INT32_TYPE),
            Field::new("dept", STRING_TYPE),
        ]);
        let left_batch = Fuzzer::new().create_record_batch(
            &left_schema,
            vec![
                vec![
                    ScalarValue::Int32(1),
                    ScalarValue::Int32(2),
                    ScalarValue::Int32(3),
                ],
                vec![
                    ScalarValue::Utf8("Alice".into()),
                    ScalarValue::Utf8("Bob".into()),
                    ScalarValue::Utf8("Carol".into()),
                ],
            ],
        );
        let right_batch = Fuzzer::new().create_record_batch(
            &right_schema,
            vec![
                vec![
                    ScalarValue::Int32(1),
                    ScalarValue::Int32(2),
                    ScalarValue::Int32(4),
                ],
                vec![
                    ScalarValue::Utf8("Engineering".into()),
                    ScalarValue::Utf8("Sales".into()),
                    ScalarValue::Utf8("Marketing".into()),
                ],
            ],
        );

        let left_df = in_memory_df("left", left_schema, left_batch);
        let right_df = in_memory_df("right", right_schema, right_batch);
        let joined = left_df.join(
            right_df,
            JoinType::Inner,
            vec![("id".into(), "id".into())],
        );

        let ctx = ExecutionContext::new(HashMap::new());
        let batches: Vec<RecordBatch> = ctx.execute_data_frame(&joined).collect();
        assert_eq!(batches.len(), 1);
        assert_eq!(to_csv(&batches[0]), "1,Alice,Engineering\n2,Bob,Sales\n");
    }

    #[test]
    fn left_join_using_dataframe() {
        let left_schema = Schema::new(vec![
            Field::new("id", INT32_TYPE),
            Field::new("name", STRING_TYPE),
        ]);
        let right_schema = Schema::new(vec![
            Field::new("id", INT32_TYPE),
            Field::new("dept", STRING_TYPE),
        ]);
        let left_batch = Fuzzer::new().create_record_batch(
            &left_schema,
            vec![
                vec![
                    ScalarValue::Int32(1),
                    ScalarValue::Int32(2),
                    ScalarValue::Int32(3),
                ],
                vec![
                    ScalarValue::Utf8("Alice".into()),
                    ScalarValue::Utf8("Bob".into()),
                    ScalarValue::Utf8("Carol".into()),
                ],
            ],
        );
        let right_batch = Fuzzer::new().create_record_batch(
            &right_schema,
            vec![
                vec![ScalarValue::Int32(1), ScalarValue::Int32(2)],
                vec![
                    ScalarValue::Utf8("Engineering".into()),
                    ScalarValue::Utf8("Sales".into()),
                ],
            ],
        );

        let left_df = in_memory_df("left", left_schema, left_batch);
        let right_df = in_memory_df("right", right_schema, right_batch);
        let joined = left_df.join(
            right_df,
            JoinType::Left,
            vec![("id".into(), "id".into())],
        );

        let ctx = ExecutionContext::new(HashMap::new());
        let batches: Vec<RecordBatch> = ctx.execute_data_frame(&joined).collect();
        assert_eq!(batches.len(), 1);
        assert_eq!(
            to_csv(&batches[0]),
            "1,Alice,Engineering\n2,Bob,Sales\n3,Carol,null\n",
        );
    }
}
