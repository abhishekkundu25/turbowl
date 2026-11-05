//! Oracles for verification (ticket 04): independent sources of the right
//! answer, each run on the same [`Input`] and returning a [`Closure`] (a set
//! of triples and a set of [`Finding`]s), compared as sets.
//!
//! | Oracle | Runs | Judges |
//! |---|---|---|
//! | `reasonable` 0.5.0-a1 | in process (dev-dependency, `tests/support`) | the `reasonable` profile, after the [`Difference`] filters |
//! | OWL-RL ([`owlrl`]) | `uv run` in `oracles/owlrl/` | soundness of the closure against the full W3C rule set |
//! | Nemo ([`nemo`]) | `tools/nemo/nmo` on `oracles/nemo/owl2rl.rls` | both profiles' closures exactly, on another engine |
//! | HermiT ([`hermit`]) | `tools/jre` + `tools/hermit` | soundness of inferred class and property subsumptions |
//! | the W3C tests ([`w3c`]) | `data/w3c/` | the standard's own expected entailments |
//! | turtle-reasoner ([`ours`]) | the [`turtle_reasoner::Reasoner`] interface | (the system under test) |
//!
//! Tools and data are fetched by `scripts/fetch-tools.sh` and
//! `scripts/fetch-data.sh`. An oracle that is not installed returns
//! [`Outcome::Skipped`]; with `CI=true` that panics instead. While the
//! `Reasoner` is still the stub, [`ours`] returns [`Outcome::Pending`].
//!
//! Triangulation ([`triangulate`]) compares the oracles with each other on
//! every input: when two agree and a third does not, the third is examined.

pub mod cases;
mod closure;
mod compare;
mod differences;
pub mod hermit;
mod input;
pub mod nemo;
pub mod ours;
pub mod owlrl;
mod process;
mod triangulate;
pub mod w3c;

pub use closure::{Closure, Finding};
pub use compare::{Diff, Relation, compare};
pub use differences::{Difference, apply as apply_differences};
pub use input::{Document, Input};
pub use triangulate::{
    Check, KNOWN, OUR_CODES, Options, REASONABLE_CODES, Row, Status, Table, triangulate, triangulate_input,
};

use std::path::{Path, PathBuf};

/// What running an oracle gave.
#[derive(Debug, Clone)]
pub enum Outcome {
    /// The oracle ran and gave this closure.
    Ran(Closure),
    /// The oracle is not installed or cannot run here; the reason says what is missing.
    Skipped(String),
    /// The system under test is not implemented yet (the stub `Reasoner`).
    Pending(String),
    /// The oracle ran and failed (a crash, a timeout, unreadable output).
    Failed(String),
}

impl Outcome {
    /// A missing oracle: `Skipped`, or a panic when `CI=true` (an oracle missing in CI is a failure).
    pub fn skipped(oracle: &str, reason: impl Into<String>) -> Self {
        let reason = reason.into();
        if ci() {
            panic!("oracle {oracle} is not available: {reason} (CI=true makes a missing oracle fail)");
        }
        Outcome::Skipped(format!("{oracle}: {reason}"))
    }

    /// The closure, when the oracle ran.
    pub fn closure(&self) -> Option<&Closure> {
        match self {
            Outcome::Ran(c) => Some(c),
            _ => None,
        }
    }

    /// One word for a results table: `ran`, `skipped`, `pending` or `failed`.
    pub fn status(&self) -> &'static str {
        match self {
            Outcome::Ran(_) => "ran",
            Outcome::Skipped(_) => "skipped",
            Outcome::Pending(_) => "pending",
            Outcome::Failed(_) => "failed",
        }
    }
}

/// True when `CI=true`: a missing oracle fails instead of being skipped.
pub fn ci() -> bool {
    std::env::var("CI").is_ok_and(|v| v.eq_ignore_ascii_case("true"))
}

/// The workspace root (two levels above this crate).
pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/oracle sits two levels below the workspace root")
        .to_path_buf()
}

/// `tools/` under the workspace root (fetched by `scripts/fetch-tools.sh`).
pub fn tools_dir() -> PathBuf {
    workspace_root().join("tools")
}

/// `data/` under the workspace root (fetched by `scripts/fetch-data.sh`).
pub fn data_dir() -> PathBuf {
    workspace_root().join("data")
}

/// The pinned version a fetch script recorded in `<dir>/VERSION`.
pub fn fetched_version(dir: &Path) -> Option<String> {
    std::fs::read_to_string(dir.join("VERSION"))
        .ok()
        .map(|v| v.trim().to_owned())
}
