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

use datatypes::{RecordBatch, Schema};
use std::fmt;

/// A physical plan represents an executable piece of code that will produce data.
///
/// `PhysicalPlan: fmt::Display` because [`format`] prints the operator tree by
/// calling each node's `Display` (Kotlin used `toString()`); every operator
/// supplies its own one-line label.
pub trait PhysicalPlan: fmt::Display {
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
