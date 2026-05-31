//! Port of `kquery/benchmarks/src/main/kotlin/Benchmarks.kt`.
//!
//! Runs a two-stage aggregate over every `.csv` file in a directory:
//!
//! 1. For each CSV file, run a *partial* aggregate (`SELECT passenger_count,
//!    MIN/MAX/SUM(CAST(fare_amount AS double)) GROUP BY passenger_count`).
//!    Each per-file query runs in parallel.
//! 2. Re-aggregate the 12-or-so partial results in memory with a *final*
//!    aggregate (`SELECT passenger_count, MIN(min_fare), MAX(max_fare),
//!    SUM(sum_fare) GROUP BY passenger_count`) — the textbook
//!    map-reduce shape. (Kotlin writes `MIN(max_fare), MAX(min_fare),
//!    SUM(max_fare)` here; we deliberately diverge to fix that bug — see
//!    `TRANSLATION_NOTES.md → Module: benchmarks`.)
//!
//! Writes a tiny `iterations,time_millis` CSV to `$BENCH_RESULT_FILE`.
//!
//! ## Configuration (matches kquery verbatim)
//!
//! Driven by **environment variables**, same shape as the Kotlin original
//! (designed to be run from Docker):
//!
//! * `BENCH_PATH` — directory containing input `.csv` files.
//! * `BENCH_RESULT_FILE` — path to write the result-summary CSV.
//!
//! The two SQL strings (`PARTIAL_SQL`, `FINAL_SQL`) are hardcoded; the
//! Kotlin source has `BENCH_SQL_PARTIAL`/`BENCH_SQL_FINAL` env-var lines
//! commented out as a `TODO parameterize`, which we preserve verbatim.
//!
//! ## Coroutines → rayon (ARCHITECTURE.md §3.9)
//!
//! The per-file fan-out is the third (and last) of the three Kotlin
//! coroutine call sites the §3.9 cheatsheet enumerates. The Rust port uses
//! `files.into_par_iter().flat_map(...)` — same shape as `parallel_query` in
//! the `examples` crate.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write;
use std::sync::Arc;
use std::time::Instant;

use datasource::{DataSource, InMemoryDataSource};
use datatypes::{RecordBatch, SchemaConverter};
use execution::ExecutionContext;
use rayon::prelude::*;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

/// First-stage SQL: per-file partial aggregate. Hardcoded — Kotlin has the
/// matching `BENCH_SQL_PARTIAL` env-var line commented out as
/// `// TODO parameterize`.
const PARTIAL_SQL: &str = "SELECT passenger_count, \
    MIN(CAST(fare_amount AS double)) AS min_fare, \
    MAX(CAST(fare_amount AS double)) AS max_fare, \
    SUM(CAST(fare_amount AS double)) AS sum_fare \
    FROM tripdata \
    GROUP BY passenger_count";

/// Second-stage SQL: final re-aggregate over the in-memory partials.
///
/// **Divergence from Kotlin:** the Kotlin original writes
/// `MIN(max_fare), MAX(min_fare), SUM(max_fare)` here — a bug; it computes
/// the minimum of the per-file *maximums*, the maximum of the per-file
/// *minimums*, and the sum of the per-file *maximums*, none of which are
/// meaningful global aggregates. We use the textbook map-reduce shape
/// (`MIN(MIN) / MAX(MAX) / SUM(SUM)`) so the benchmark produces semantically
/// correct global numbers. See `TRANSLATION_NOTES.md → Module: benchmarks`.
const FINAL_SQL: &str = "SELECT passenger_count, \
    MIN(min_fare), MAX(max_fare), SUM(sum_fare) \
    FROM tripdata \
    GROUP BY passenger_count";

fn main() {
    env_logger::init();

    // ---- Memory stats: BEFORE ----
    // Kotlin: `Runtime.getRuntime().{max,total,free}Memory()`. The Rust
    // analogue is `sysinfo` — see TRANSLATION_NOTES standing convention.
    // We report this process's RSS (resident set size) + virtual memory + the
    // host's total memory, which is the closest set of three to Kotlin's trio.
    print_memory_stats("before");

    let path = env::var("BENCH_PATH")
        .expect("BENCH_PATH env var required (input CSV directory)");
    let result_file = env::var("BENCH_RESULT_FILE")
        .expect("BENCH_RESULT_FILE env var required (output result CSV)");

    // Kotlin: `mapOf(Pair("kquery.csv.batchSize", "1024"))` — verbatim
    // except the key is renamed (per TRANSLATION_NOTES kquery→rquery
    // rename of user-visible strings).
    let mut settings = HashMap::new();
    settings.insert("rquery.csv.batchSize".to_string(), "1024".to_string());

    sql_aggregate(&path, PARTIAL_SQL, FINAL_SQL, &result_file, settings);

    // ---- Memory stats: AFTER ----
    print_memory_stats("after");
}

/// The two-stage aggregate. Mirrors Kotlin's `sqlAggregate(path, sqlPartial,
/// sqlFinal, resultFile, settings)`.
fn sql_aggregate(
    path: &str,
    sql_partial: &str,
    sql_final: &str,
    result_file: &str,
    settings: HashMap<String, String>,
) {
    let start = Instant::now();

    // List CSV files in `path`. Kotlin: `File(path).list().filter { it.endsWith(".csv") }`.
    let files = list_csv_files(path);

    // -----------------------------------------------------------------------
    // First stage: one partial-aggregate query per CSV file, fanned out in
    // parallel. This is the kquery `GlobalScope.async { … }` site — replaced
    // with rayon per ARCHITECTURE.md §3.9.
    // -----------------------------------------------------------------------
    let results: Vec<RecordBatch> = files
        .into_par_iter()
        .flat_map(|file| {
            let full_path = format!("{path}/{file}");
            println!("Executing query against {file} ...");
            let partition_start = Instant::now();
            let batches = execute_query(&full_path, sql_partial, &settings);
            let duration = partition_start.elapsed().as_millis();
            println!("Query against {file} took {duration} ms");
            batches
        })
        .collect();

    let first = results
        .first()
        .expect("no result batches collected — is BENCH_PATH empty of .csv files?");
    println!("{:?}", first.schema());

    // -----------------------------------------------------------------------
    // Second stage: register the partials as an InMemoryDataSource and run
    // the final aggregate over them.
    // -----------------------------------------------------------------------
    let final_schema = SchemaConverter::from_arrow(&first.schema());
    let in_memory: Arc<dyn DataSource> =
        Arc::new(InMemoryDataSource::new(final_schema, results));

    let mut ctx = ExecutionContext::new(settings);
    ctx.register_data_source("tripdata", in_memory);

    let df = ctx.sql(sql_final);
    for batch in ctx.execute_data_frame(&df) {
        // Kotlin: `println(it)`. We use `{:?}` to match Kotlin's
        // `RecordBatch.toString()`-style verbose dump; if you want CSV rows
        // instead, swap for `to_csv(&batch)` (same convention as `nyc_taxi`).
        println!("{batch:?}");
    }

    let duration = start.elapsed().as_millis();
    println!("Executed query in {duration} ms");

    // -----------------------------------------------------------------------
    // Write the result-summary CSV. Same two-line shape as Kotlin's
    // `FileWriter` block (header line + one data line).
    // -----------------------------------------------------------------------
    let mut w = fs::File::create(result_file).unwrap_or_else(|e| {
        panic!("cannot create BENCH_RESULT_FILE '{result_file}': {e}")
    });
    writeln!(w, "iterations,time_millis").expect("write header");
    writeln!(w, "1,{duration}").expect("write row");
}

/// Per-file partial-query worker. Mirrors Kotlin's `executeQuery`.
fn execute_query(
    path: &str,
    sql: &str,
    settings: &HashMap<String, String>,
) -> Vec<RecordBatch> {
    let mut ctx = ExecutionContext::new(settings.clone());
    ctx.register_csv("tripdata", path);
    let df = ctx.sql(sql);
    ctx.execute_data_frame(&df).collect()
}

/// List every `.csv` file in `path`, returning the bare file names (no
/// directory prefix). Mirrors Kotlin's `dir.list().filter { it.endsWith(".csv") }`.
fn list_csv_files(path: &str) -> Vec<String> {
    let entries = fs::read_dir(path)
        .unwrap_or_else(|e| panic!("cannot read BENCH_PATH '{path}': {e}"));
    let mut out: Vec<String> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".csv"))
        .collect();
    out.sort(); // stable order; Kotlin's File.list ordering is platform-defined
    out
}

/// Print process-memory stats — Rust analogue of Kotlin's
/// `Runtime.getRuntime().{max,total,free}Memory()`. Uses `sysinfo` (per
/// TRANSLATION_NOTES standing convention). The three numbers we print are
/// the closest analogues:
///   * `max`   → host total memory (the JVM `-Xmx` analogue)
///   * `total` → process virtual memory
///   * `free`  → host available memory
fn print_memory_stats(label: &str) {
    let mut sys = System::new();
    sys.refresh_memory();
    // sysinfo 0.31: refresh_processes_specifics takes (ProcessesToUpdate, ProcessRefreshKind);
    // ProcessRefreshKind::new() starts empty, .with_memory() opts memory in.
    let pid = Pid::from_u32(std::process::id());
    sys.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        ProcessRefreshKind::new().with_memory(),
    );
    let process_virtual = sys.process(pid).map(|p| p.virtual_memory()).unwrap_or(0);
    println!(
        "[{label}] maxMemory={} totalMemory={} freeMemory={}",
        sys.total_memory(),
        process_virtual,
        sys.available_memory(),
    );
}
