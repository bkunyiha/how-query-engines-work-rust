//! Port of `kquery/examples/src/main/kotlin/NYCTaxi.kt`.
//!
//! Reads a single month of the NYC yellow-taxi trip data and runs:
//!
//! ```sql
//! SELECT passenger_count, MAX(CAST(fare_amount AS float))
//!   FROM tripdata
//!   GROUP BY passenger_count
//! ```
//!
//! Prints the logical plan, the optimized plan, every result batch, and the
//! wall-clock time. This is the **definition-of-done** binary called out in
//! `README.md`: a successful run on the same input file should produce output
//! matching the kquery reference (modulo float-formatting, see
//! `TRANSLATION_NOTES.md → Module: execution`).
//!
//! ## Where the input file lives
//! The path is **hardcoded** to match the kquery original verbatim
//! (`TRANSLATION_NOTES.md → Module: examples` records this as a deliberate
//! Phase-1 choice — faithful port, NYC file paths kept literal). Obtain the
//! file once with:
//!
//! ```text
//! wget https://s3.amazonaws.com/nyc-tlc/trip+data/yellow_tripdata_2019-01.csv
//! ```
//!
//! and place / symlink it at the path below. Without the file, the binary
//! panics with the `CsvDataSource` "file not found" error — same observable
//! behaviour as the Kotlin original.

use std::collections::HashMap;
use std::time::Instant;

use datatypes::RecordBatch;
use datatypes::arrow_types::FLOAT_TYPE;
use datatypes::record_batch::to_csv;
use execution::ExecutionContext;
use logical_plan::{cast, col, format, max};
use optimizer::optimizer::Optimizer;

/// Hardcoded NYC yellow-taxi 2019-01 path. Matches `NYCTaxi.kt` verbatim; see
/// the module-doc for how to obtain the file.
const NYC_TAXI_CSV: &str = "/mnt/nyctaxi/csv/year=2019/yellow_tripdata_2019-01.csv";

fn main() {
    env_logger::init();

    let ctx = ExecutionContext::new(HashMap::new());

    let start = Instant::now();

    // SELECT passenger_count, MAX(CAST(fare_amount AS float)) GROUP BY passenger_count
    let df = ctx.csv(NYC_TAXI_CSV).aggregate(
        vec![col("passenger_count")],
        vec![max(cast(col("fare_amount"), FLOAT_TYPE))],
    );

    println!("Logical Plan:\t{}", format(df.logical_plan()));

    // Print the optimized plan separately so a reader can see what
    // `ProjectionPushDown` (and other rules) do to the logical tree.
    // `ExecutionContext::execute()` will re-run `Optimizer::optimize` internally;
    // the optimizer is idempotent, so the second pass is a no-op shape-wise.
    // This mirrors the Kotlin original's two-step (manual optimize + execute).
    let optimized_plan = Optimizer::new().optimize(df.logical_plan());
    println!("Optimized Plan:\t{}", format(&optimized_plan));

    let results: Box<dyn Iterator<Item = RecordBatch>> = ctx.execute(df.logical_plan());
    for batch in results {
        // Print each batch's schema and its CSV rendering. Kotlin prints
        // `batch.schema` (the kquery `Schema` data class's `toString`); the
        // Rust equivalent here is the arrow-rs `Schema`'s `Debug` form, which
        // is what `RecordBatch::schema()` returns under our `datatypes` crate.
        println!("{:?}", batch.schema());
        println!("{}", to_csv(&batch));
    }

    println!("Query took {} ms", start.elapsed().as_millis());
}
