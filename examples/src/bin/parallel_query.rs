//! Port of `kquery/examples/src/main/kotlin/ParallelQuery.kt`.
//!
//! Fans out 12 monthly CSV queries — one per month of the NYC yellow-taxi 2019
//! data — runs them in parallel, then re-aggregates the 12 result vectors into
//! a single final result. The Kotlin original drives the fan-out with
//! `kotlinx.coroutines.GlobalScope.async { … }.await`; the Rust port uses
//! **rayon** (the faithful substitution from `ARCHITECTURE.md §3.9` — this is
//! one of the three coroutine call sites the cheatsheet enumerates).
//!
//! ## Where the input files live
//! The directory path is **hardcoded** to match `ParallelQuery.kt` verbatim
//! (see `TRANSLATION_NOTES.md → Module: examples`). Twelve files are expected
//! at `${PATH}/yellow_tripdata_2019-{01..12}.csv`. Without them, the per-month
//! query panics inside `CsvDataSource` — same observable behaviour as the
//! Kotlin original.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use datasource::InMemoryDataSource;
use datatypes::{RecordBatch, SchemaConverter};
use datatypes::record_batch::to_csv;
use execution::ExecutionContext;
use rayon::prelude::*;

/// Hardcoded directory holding `yellow_tripdata_2019-{01..12}.csv`. Matches
/// `ParallelQuery.kt` verbatim; see the module-doc for how to obtain the files.
const PATH: &str = "/mnt/nyctaxi/csv/yellow/2019";

/// SQL each per-month worker runs. The aliased `max_fare` flows into the final
/// re-aggregation below.
const PER_MONTH_SQL: &str = "SELECT passenger_count, \
                             MAX(CAST(fare_amount AS double)) AS max_fare \
                             FROM tripdata \
                             GROUP BY passenger_count";

/// SQL for the final re-aggregation over the 12 per-month partial results.
const FINAL_SQL: &str = "SELECT passenger_count, MAX(max_fare) \
                         FROM tripdata GROUP BY passenger_count";

fn main() {
    env_logger::init();

    let start = Instant::now();

    // -----------------------------------------------------------------------
    // Fan-out: 12 months × per-month query, run in parallel via rayon.
    //
    // Kotlin: `(1..12).map { month -> GlobalScope.async { executeQuery(...) } }
    //          .flatMap { it.await() }`
    // Rust:   `(1..=12).into_par_iter().flat_map(...)` — same semantics, same
    //         CPU-bound work, just spelled in rayon (ARCHITECTURE §3.9).
    // -----------------------------------------------------------------------
    let results: Vec<RecordBatch> = (1u32..=12)
        .into_par_iter()
        .flat_map(|month| {
            let part_start = Instant::now();
            let batches = execute_query(PATH, month, PER_MONTH_SQL);
            println!(
                "Query against month {month} took {} ms",
                part_start.elapsed().as_millis()
            );
            batches
        })
        .collect();

    let duration = start.elapsed().as_millis();
    println!("Collected {} batches in {duration} ms", results.len());

    let first = results.first().expect("no result batches collected");
    println!("{:?}", first.schema());

    // -----------------------------------------------------------------------
    // Re-aggregate the 12 per-month partials. Register the collected batches
    // as an InMemoryDataSource and run the FINAL_SQL through a fresh context.
    //
    // `RecordBatch::schema()` returns an `Arc<arrow_schema::Schema>`;
    // `InMemoryDataSource::new` wants the kquery-style `datatypes::Schema`, so
    // we round-trip through `SchemaConverter::from_arrow`.
    // -----------------------------------------------------------------------
    let final_schema = SchemaConverter::from_arrow(&first.schema());
    let in_memory: Arc<dyn datasource::DataSource> =
        Arc::new(InMemoryDataSource::new(final_schema, results));

    let mut ctx = ExecutionContext::new(HashMap::new());
    ctx.register_data_source("tripdata", in_memory);

    let df = ctx.sql(FINAL_SQL);
    for batch in ctx.execute_data_frame(&df) {
        // `println!("{batch:?}")` would dump arrow-rs's verbose Debug; `to_csv`
        // gives the same row-per-line view as Kotlin's `println(batch)`
        // produces from its `RecordBatch.toString()`. More readable, same info.
        print!("{}", to_csv(&batch));
    }
}

/// Per-month worker. Builds a fresh `ExecutionContext`, registers
/// `yellow_tripdata_2019-{MM}.csv` under the table name `tripdata`, runs `sql`,
/// and returns the collected batches. Mirrors Kotlin's `executeQuery`.
///
/// A fresh context per call is the simplest faithful translation; the Kotlin
/// original does the same. Each worker therefore owns its own table registry
/// and CSV reader — no shared mutable state across rayon workers, which keeps
/// the closure trivially `Send` without any wrapping.
fn execute_query(path: &str, month: u32, sql: &str) -> Vec<RecordBatch> {
    let filename = format!("{path}/yellow_tripdata_2019-{month:02}.csv");
    let mut ctx = ExecutionContext::new(HashMap::new());
    ctx.register_csv("tripdata", &filename);
    let df = ctx.sql(sql);
    ctx.execute_data_frame(&df).collect()
}
