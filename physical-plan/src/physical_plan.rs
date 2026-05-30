//! Port of `kquery/physical-plan/src/main/kotlin/PhysicalPlan.kt`.
//!
//! A physical plan is an executable piece of code that produces data. It is the
//! runtime counterpart of a [`logical_plan::LogicalPlan`]: the logical plan says
//! *what* to compute, the physical plan says *how* and actually runs it.
//!
//! ## Translation note — trait, not enum (ARCHITECTURE.md §4.6)
//! Everywhere else in this port a Kotlin interface with a closed implementor set
//! becomes a Rust `enum` (§3.1 — see `logical_plan::LogicalPlan`, which has six
//! variants). `PhysicalPlan` is the documented exception. The physical layer is
//! the largest module in the workspace (28 Kotlin files) and the operator set is
//! *open in spirit*: adding a new operator should mean adding a new file, not
//! editing a central enum and every `match` over it. The Kotlin source models
//! this with inheritance (`class ProjectionExec : PhysicalPlan`), so the faithful
//! Rust port uses a **trait** referenced through `Box<dyn PhysicalPlan>`. The
//! deviation is intentional and recorded in `TRANSLATION_NOTES.md`.
//!
//! ## Translation note — `Sequence` → `Box<dyn Iterator>`
//! Kotlin's `execute(): Sequence<RecordBatch>` is a lazy stream. The Rust analogue
//! is a boxed iterator, `Box<dyn Iterator<Item = RecordBatch>>`. (A future async
//! rewrite would return a `futures::Stream`; this faithful port stays synchronous,
//! matching the Kotlin shape.)
//!
//! ## Translation note — `Send + Sync` for parallel execution (Module 8)
//! `PhysicalPlan` (and its sibling expression traits) require `Send + Sync`.
//! Kotlin's `ParallelContext` runs partial aggregates on multiple workers via
//! coroutines (`runBlocking { async { … } }`); the faithful Rust substitution is
//! `rayon` (ARCHITECTURE.md §3.9). rayon moves work onto a worker pool, so every
//! value a worker touches — the `Box<dyn PhysicalPlan>` it runs and the
//! `Arc<dyn Expression>` / `Arc<dyn AggregateExpression>` it shares — must be
//! `Send + Sync`. Adding the bound here propagates to `Expression`,
//! `AggregateExpression` (physical-plan) and `DataSource` (datasource). It is
//! satisfied automatically: every operator/expression holds only `Send + Sync`
//! data (arrow `ArrayRef = Arc<dyn Array>` is `Send + Sync`, and the structs carry
//! `Arc`/`Box` of these same traits plus plain data). The bound is also a
//! prerequisite for the distributed/Flight modules (13–15), which serve batches
//! across threads. `ColumnVector` and `Accumulator` deliberately do *not* gain the
//! bound — their trait objects are created and consumed inside a single worker and
//! never cross a thread boundary.

use crate::hash_aggregate_exec::HashAggregateExec;
use datatypes::{RecordBatch, Schema};
use std::fmt;

/// A physical plan represents an executable piece of code that will produce data.
///
/// `PhysicalPlan: fmt::Display` because [`format`] prints the operator tree by
/// calling each node's `Display` (Kotlin used `toString()`); every operator
/// supplies its own one-line label. `Send + Sync` lets `ParallelContext` hand
/// plans to rayon workers (see the module-level note).
pub trait PhysicalPlan: fmt::Display + Send + Sync {
    /// The schema of the data this plan produces.
    fn schema(&self) -> Schema;

    /// Execute the plan and produce a (lazy) series of record batches.
    fn execute(&self) -> Box<dyn Iterator<Item = RecordBatch>>;

    /// The children (inputs) of this plan, used to walk the operator tree.
    ///
    /// Returns borrowed references rather than owned/cloned nodes: a parent owns
    /// its child as `Box<dyn PhysicalPlan>`, and the tree walk only needs to read
    /// them. Leaf operators (e.g. a scan) return an empty vec.
    fn children(&self) -> Vec<&dyn PhysicalPlan>;

    /// Typed downcast hook used by `ParallelContext` to special-case parallel
    /// aggregation. Kotlin's `executeParallel` pattern-matches the physical plan
    /// (`when (plan) { is HashAggregateExec -> … }`), but Rust cannot match a
    /// `&dyn PhysicalPlan` by concrete type without `Any`. Rather than add
    /// `as_any` boilerplate to every operator, the trait exposes this accessor:
    /// it defaults to `None` and is overridden *only* by [`HashAggregateExec`].
    /// The coupling to one concrete operator is intentional and contained — it
    /// keeps the special case to a single line at the call site.
    fn as_hash_aggregate(&self) -> Option<&HashAggregateExec> {
        None
    }

    /// Human-readable, indented rendering of this plan and its subtree.
    /// Kotlin: `PhysicalPlan.pretty()`.
    fn pretty(&self) -> String
    where
        Self: Sized,
    {
        format(self)
    }
}

/// Format a physical plan in human-readable form: one line per node, indented by
/// depth with tabs. Kotlin: the private top-level `format(plan, indent)`.
pub fn format(plan: &dyn PhysicalPlan) -> String {
    fn go(plan: &dyn PhysicalPlan, indent: usize, out: &mut String) {
        for _ in 0..indent {
            out.push('\t');
        }
        out.push_str(&plan.to_string());
        out.push('\n');
        for child in plan.children() {
            go(child, indent + 1, out);
        }
    }
    let mut out = String::new();
    go(plan, 0, &mut out);
    out
}
