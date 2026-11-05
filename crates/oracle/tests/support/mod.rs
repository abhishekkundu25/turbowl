//! The `reasonable` oracle: the pinned crates.io release 0.5.0-a1 (published
//! from commit fecee2b, the commit the parity research read), run in process.
//! It is a dev-dependency, so it lives with the tests. Each test binary uses
//! part of this module.
#![allow(dead_code)]

pub mod data;

use std::panic::{AssertUnwindSafe, catch_unwind};
use turtle_reasoner_oracle::{Closure, Finding, Input, Outcome};

/// Loads the input's triples (blank nodes already scoped per document by
/// `Input`, so D4 cannot show), reasons, and reads the closure and the
/// diagnostics (their terms parsed from the messages).
pub fn reasonable(input: &Input) -> Outcome {
    let run = catch_unwind(AssertUnwindSafe(|| {
        let mut r = reasonable::reasoner::Reasoner::new();
        r.load_triples(input.triples().to_vec());
        r.reason();
        let findings: Vec<Finding> = r
            .errors()
            .iter()
            .map(|e| Finding::from_reasonable(e.code(), e.message()))
            .collect();
        Closure::new(r.get_triples(), findings)
    }));
    match run {
        Ok(c) => Outcome::Ran(c),
        Err(p) => Outcome::Failed(format!(
            "reasonable panicked: {}",
            p.downcast_ref::<String>()
                .cloned()
                .or_else(|| p.downcast_ref::<&str>().map(|s| (*s).to_owned()))
                .unwrap_or_default()
        )),
    }
}

/// Where a test writes its report: `target/oracle/<name>.md`.
pub fn report(name: &str, text: &str) -> std::path::PathBuf {
    let path = turtle_reasoner_oracle::workspace_root()
        .join("target/oracle")
        .join(format!("{name}.md"));
    std::fs::create_dir_all(path.parent().expect("has a parent")).expect("target/oracle is writable");
    std::fs::write(&path, text).expect("report is writable");
    path
}
