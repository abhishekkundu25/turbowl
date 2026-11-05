//! Oracles for verification (ticket 04, retargeted by ADR 0004): independent
//! sources of the right answer under the W3C OWL 2 RL rules, for the rule
//! set of version 1 (`docs/specs/v1.md`). Each runs on the same [`Input`] and
//! returns a [`Closure`] (a set of triples and a set of [`Finding`]s); they
//! are compared as sets.
//!
//! | Oracle | Runs | Judges |
//! |---|---|---|
//! | Nemo ([`nemo`]) | `tools/nemo/nmo` on the program [`programs`] composes from `oracles/rules/` | the closure and the findings exactly (dt-not-type aside), on another engine |
//! | OWL-RL ([`owlrl`]) | `uv run` in `oracles/owlrl/` | containment: it implements more rules, so ours ⊆ OWL-RL after the [`Fault`] filters |
//! | HermiT ([`hermit`]) | `tools/jre` + `tools/hermit` | soundness of inferred class and property subsumptions |
//! | the W3C tests ([`w3c`]) | `data/w3c/` | the standard's own expected entailments; every case Nemo passes is required of us |
//! | the reference evaluator | `crates/reference` (`tests/reference.rs`) | itself, against Nemo, on generated inputs |
//! | TurbOWL ([`ours`]) | the [`turbowl::Reasoner`] interface | (the system under test) |
//!
//! The competitors (`CONTEXT.md`) are measured for speed and memory in
//! `crates/bench`; only the oracles above judge correctness. The Datalog
//! engines among them run the programs [`programs`] emits from the same
//! rule source as the Nemo oracle.
//!
//! Tools and data are fetched by `scripts/fetch-tools.sh` and
//! `scripts/fetch-data.sh`. An oracle that is not installed returns
//! [`Outcome::Skipped`]; with `CI=true` that panics instead.
//!
//! Triangulation ([`triangulate`]) compares the oracles with each other on
//! every input: when two agree and a third does not, the third is examined,
//! and a confirmed error of an oracle becomes an oracle fault ([`Fault`]).

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
pub mod programs;
mod triangulate;
pub mod w3c;

pub use closure::{CODES, Closure, Finding, codes, triple};
pub use compare::{Diff, Relation, compare};
pub use differences::{Fault, apply as apply_faults, explains_failure};
pub use input::{Document, Input, NAMED_LITERALS};
use std::path::{Path, PathBuf};
pub use triangulate::{Check, KNOWN, Options, Row, Status, Table, configurations, triangulate, triangulate_input};

/// What running an oracle gave.
#[derive(Debug, Clone)]
pub enum Outcome {
    /// The oracle ran and gave this closure.
    Ran(Closure),
    /// The oracle is not installed or cannot run here, or the input is
    /// outside what it judges; the reason says which.
    Skipped(String),
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

    /// One word for a results table: `ran`, `skipped` or `failed`.
    pub fn status(&self) -> &'static str {
        match self {
            Outcome::Ran(_) => "ran",
            Outcome::Skipped(_) => "skipped",
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
