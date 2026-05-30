//! Port of `kquery/execution/src/main/kotlin/ParallelContext.kt`.
//!
//! An execution context that runs aggregate queries in parallel. For a
//! `HashAggregateExec` it: (1) collects the input batches and distributes them
//! round-robin across workers, (2) runs a *partial* aggregate on each worker's
//! slice in parallel, then (3) merges the partial results with a *final*
//! aggregate. Non-aggregate plans fall through to ordinary sequential execution.
//!
//! ## Translation notes
//! - **Kotlin coroutines → rayon (ARCHITECTURE.md §3.9 / §4.8).** Kotlin spawns one
//!   `async { … }` per worker queue inside `runBlocking(Dispatchers.Default)` and
//!   awaits them. The work (`HashAggregateExec.execute()`) is **CPU-bound** — it
//!   walks `ColumnVector`s and folds accumulators — so the faithful Rust analogue
//!   is `rayon` (a work-stealing pool for CPU-bound closures), **not** `tokio`
//!   (which targets I/O-bound concurrency and would starve its reactor here). The
//!   round-robin `worker_batches` buckets are mapped in parallel with
//!   `into_par_iter()`.
//! - **`Send + Sync` prerequisite.** rayon moves each bucket onto a worker and
//!   shares `&HashAggregateExec` across threads, so `PhysicalPlan` / `Expression` /
//!   `AggregateExpression` / `DataSource` carry `Send + Sync` bounds (added for this
//!   module; see the `physical_plan` module note). The cloned `group_expr` /
//!   `aggregate_expr` (`Arc` clones) and the schema all satisfy them.
//! - **Concrete-type recovery.** Kotlin's `executeParallel` does
//!   `when (plan) { is HashAggregateExec -> … else -> plan.execute() }`. Rust can't
//!   match a `&dyn PhysicalPlan` by concrete type, so [`PhysicalPlan`] exposes an
//!   `as_hash_aggregate()` downcast hook (default `None`, overridden by
//!   `HashAggregateExec`) — see its trait docs.
//! - **`InMemoryPlan`** mirrors Kotlin's private same-named class: a leaf physical
//!   plan that simply replays a pre-loaded `Vec<RecordBatch>`, used to feed the
//!   partial and final aggregates.
//! - **`ConcurrentLinkedQueue` → `Vec<Vec<RecordBatch>>`.** The Kotlin code uses a
//!   thread-safe queue per worker only because it builds the buckets and reads them
//!   from coroutines; here the buckets are filled on one thread before the parallel
//!   phase, so plain `Vec`s suffice.

use std::collections::HashMap;
use std::fmt;
use std::num::NonZeroUsize;
use std::sync::Arc;

use rayon::prelude::*;

use datasource::{CsvDataSource, DataSource};
use datatypes::{RecordBatch, Schema};
use logical_plan::{DataFrame, LogicalPlan, Scan};
use optimizer::optimizer::Optimizer;
use physical_plan::{AggregateMode, HashAggregateExec, PhysicalPlan};
use query_planner::QueryPlanner;
use sql::expressions::SqlExpr;
use sql::pratt_parser::PrattParser; // brings the `parse` method into scope
use sql::sql_parser::SqlParser;
use sql::sql_planner::SqlPlanner;
use sql::sql_tokenizer::SqlTokenizer;

/// Default CSV batch size when `rquery.csv.batchSize` is unset. Kotlin: `"1024"`.
const DEFAULT_BATCH_SIZE: usize = 1024;

/// Number of workers when none is given. Kotlin: `Runtime.availableProcessors()`.
fn default_parallelism() -> usize {
    std::thread::available_parallelism().map(NonZeroUsize::get).unwrap_or(1)
}

/// Execution context with parallel aggregation. Kotlin `class ParallelContext`.
pub struct ParallelContext {
    /// Number of parallel workers. Kotlin `val parallelism`.
    pub parallelism: usize,
    /// Configuration settings. Kotlin `val settings`.
    pub settings: HashMap<String, String>,
    batch_size: usize,
    tables: HashMap<String, DataFrame>,
}

impl Default for ParallelContext {
    fn default() -> Self {
        Self::with_parallelism(default_parallelism(), HashMap::new())
    }
}

impl ParallelContext {
    /// Parallelism defaults to the available CPU count; empty settings. Kotlin's
    /// `ParallelContext()` with both parameters defaulted.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct with an explicit worker count. Kotlin
    /// `ParallelContext(parallelism = …, settings = …)`.
    pub fn with_parallelism(parallelism: usize, settings: HashMap<String, String>) -> Self {
        let batch_size = settings
            .get("rquery.csv.batchSize")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(DEFAULT_BATCH_SIZE);
        Self { parallelism, settings, batch_size, tables: HashMap::new() }
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

    /// Register a `DataFrame` with the context. Kotlin `register`.
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

    /// Execute the provided logical plan with parallel processing. Kotlin
    /// `execute(plan)`.
    pub fn execute(&self, plan: &LogicalPlan) -> Box<dyn Iterator<Item = RecordBatch>> {
        let optimized = Optimizer::new().optimize(plan);
        let physical = QueryPlanner::new().create_physical_plan(&optimized);
        self.execute_parallel(physical.as_ref())
    }

    /// Run a physical plan, special-casing `HashAggregateExec` for parallelism.
    /// Kotlin `executeParallel`.
    fn execute_parallel(&self, plan: &dyn PhysicalPlan) -> Box<dyn Iterator<Item = RecordBatch>> {
        match plan.as_hash_aggregate() {
            Some(aggregate) => self.execute_parallel_aggregate(aggregate),
            None => plan.execute(),
        }
    }

    /// Parallel partial/final aggregation. Kotlin `executeParallelAggregate`.
    fn execute_parallel_aggregate(
        &self,
        aggregate: &HashAggregateExec,
    ) -> Box<dyn Iterator<Item = RecordBatch>> {
        // With a single worker there is nothing to fan out — run it directly.
        if self.parallelism <= 1 {
            return aggregate.execute();
        }

        // Collect the input batches and distribute them round-robin to workers.
        let input_batches: Vec<RecordBatch> = aggregate.input.execute().collect();
        let mut worker_batches: Vec<Vec<RecordBatch>> =
            (0..self.parallelism).map(|_| Vec::new()).collect();
        for (index, batch) in input_batches.into_iter().enumerate() {
            worker_batches[index % self.parallelism].push(batch);
        }

        // Run a partial aggregate per non-empty bucket, in parallel (rayon is the
        // CPU-bound substitute for Kotlin's coroutines — see the module note).
        let partial_results: Vec<RecordBatch> = worker_batches
            .into_par_iter()
            .filter(|bucket| !bucket.is_empty())
            .map(|bucket| execute_partial_aggregate(aggregate, bucket))
            .collect::<Vec<Vec<RecordBatch>>>()
            .into_iter()
            .flatten()
            .collect();

        if partial_results.is_empty() {
            return Box::new(std::iter::empty::<RecordBatch>());
        }

        // Merge the partial results with a final aggregate.
        execute_final_aggregate(aggregate, partial_results)
    }
}

/// Run a `Partial` aggregate over one worker's batches. Kotlin
/// `executePartialAggregate`. A free function (not a method) so the rayon closure
/// captures only `&HashAggregateExec`, never `&self`.
fn execute_partial_aggregate(
    aggregate: &HashAggregateExec,
    batches: Vec<RecordBatch>,
) -> Vec<RecordBatch> {
    let partial = HashAggregateExec::new_with_mode(
        Box::new(InMemoryPlan::new(aggregate.input.schema(), batches)),
        aggregate.group_expr.clone(),
        aggregate.aggregate_expr.clone(),
        aggregate.schema.clone(),
        AggregateMode::Partial,
    );
    partial.execute().collect()
}

/// Merge the partial results with a `Final` aggregate. Kotlin
/// `executeFinalAggregate`. Note the input schema is the *aggregate's* output
/// schema (the partial results), not the original input schema.
fn execute_final_aggregate(
    aggregate: &HashAggregateExec,
    partial_batches: Vec<RecordBatch>,
) -> Box<dyn Iterator<Item = RecordBatch>> {
    let final_aggregate = HashAggregateExec::new_with_mode(
        Box::new(InMemoryPlan::new(aggregate.schema.clone(), partial_batches)),
        aggregate.group_expr.clone(),
        aggregate.aggregate_expr.clone(),
        aggregate.schema.clone(),
        AggregateMode::Final,
    );
    final_aggregate.execute()
}

/// Leaf physical plan that replays pre-loaded batches. Kotlin's private
/// `InMemoryPlan`. Used to feed batches into the partial/final aggregates.
struct InMemoryPlan {
    schema: Schema,
    batches: Vec<RecordBatch>,
}

impl InMemoryPlan {
    fn new(schema: Schema, batches: Vec<RecordBatch>) -> Self {
        Self { schema, batches }
    }
}

impl fmt::Display for InMemoryPlan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "InMemoryPlan: {} batches", self.batches.len())
    }
}

impl PhysicalPlan for InMemoryPlan {
    fn schema(&self) -> Schema {
        self.schema.clone()
    }

    fn children(&self) -> Vec<&dyn PhysicalPlan> {
        Vec::new()
    }

    fn execute(&self) -> Box<dyn Iterator<Item = RecordBatch>> {
        // arrow `RecordBatch` is `Arc`-backed, so cloning the vec is cheap.
        Box::new(self.batches.clone().into_iter())
    }
}

#[cfg(test)]
mod tests {
    //! Port of `ParallelContextTest.kt`. Compares the parallel context against the
    //! sequential `ExecutionContext` on a GROUP BY / SUM query, and checks that a
    //! single-worker parallel context still produces results.
    use super::*;
    use crate::execution_context::ExecutionContext;
    use datatypes::record_batch::{row_count, to_csv};
    use std::collections::HashSet;

    const EMPLOYEE_CSV: &str = "../testdata/employee.csv";
    const SQL: &str = "SELECT state, SUM(CAST(salary AS double)) FROM employee GROUP BY state";

    /// Flatten batches into a set of CSV rows so comparisons ignore the
    /// HashMap-driven output order.
    fn row_set(batches: &[RecordBatch]) -> HashSet<String> {
        batches
            .iter()
            .flat_map(|b| to_csv(b).lines().map(str::to_string).collect::<Vec<_>>())
            .collect()
    }

    #[test]
    fn parallel_aggregate_matches_sequential() {
        let mut seq = ExecutionContext::new(HashMap::new());
        seq.register_csv("employee", EMPLOYEE_CSV);
        let seq_df = seq.sql(SQL);
        let seq_rows = row_set(&seq.execute_data_frame(&seq_df).collect::<Vec<_>>());

        let mut par = ParallelContext::with_parallelism(4, HashMap::new());
        par.register_csv("employee", EMPLOYEE_CSV);
        let par_df = par.sql(SQL);
        let par_rows = row_set(&par.execute_data_frame(&par_df).collect::<Vec<_>>());

        assert!(!seq_rows.is_empty(), "sequential produced no rows");
        assert_eq!(seq_rows, par_rows);
    }

    #[test]
    fn parallelism_one_behaves_like_sequential() {
        let mut ctx = ParallelContext::with_parallelism(1, HashMap::new());
        ctx.register_csv("employee", EMPLOYEE_CSV);
        let df = ctx.sql(SQL);
        let batches: Vec<RecordBatch> = ctx.execute_data_frame(&df).collect();
        assert_eq!(batches.len(), 1);
        assert!(row_count(&batches[0]) > 0);
    }
}
