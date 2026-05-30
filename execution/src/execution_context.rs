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
use optimizer::optimizer::Optimizer;
use query_planner::QueryPlanner;
use sql::expressions::SqlExpr;
use sql::pratt_parser::PrattParser; // brings the `parse` method into scope
use sql::sql_parser::SqlParser;
use sql::sql_planner::SqlPlanner;
use sql::sql_tokenizer::SqlTokenizer;

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
    pub fn execute(&self, plan: &LogicalPlan) -> Box<dyn Iterator<Item = RecordBatch>> {
        let optimized = Optimizer::new().optimize(plan);
        let physical = QueryPlanner::new().create_physical_plan(&optimized);
        physical.execute()
    }
}

#[cfg(test)]
mod tests {
    //! Port of `ExecutionSqlTest.kt` (logical-plan-string assertions) and the
    //! deterministic, non-`Fuzzer` cases of `ExecutionTest.kt`. The `Fuzzer`-backed
    //! cases (`min max sum float`, `float math`, `boolean expressions`) belong to
    //! module 9 (`fuzzer`); the `date and interval arithmetic` case exercises the
    //! `LiteralDate` path the planner deliberately `panic!`s on today (see the
    //! `query-planner` notes), so it is intentionally omitted here.
    use super::*;
    use datatypes::arrow_types::INT32_TYPE;
    use datatypes::record_batch::to_csv;
    use logical_plan::{cast, col, format, lit_string, max};
    use std::collections::HashSet;

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
}
