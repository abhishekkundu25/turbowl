//! Writes the minimal inputs of the oracle suite and the Rust library's
//! results on them, for the Python tests (`tests/test_minimal.py`) to
//! compare with what the bindings give.
//!
//! ```sh
//! cargo run --release --locked -p turbowl-python --example minimal_cases -- <dir>
//! ```
//!
//! Into `<dir>`: `cases.txt`, the case names in order, and for each case
//! `<name>.input.nt`, the input triples, blank nodes labelled as
//! [`turbowl_oracle::Input`] labels them. Then, for each configuration, the
//! default ([`Switches::default`]) and the complete one
//! ([`Switches::complete`]), into `<dir>/default/` and `<dir>/complete/`:
//!
//! - `<name>.closure.nt` and `<name>.inferred.nt`: [`Reasoner::triples`] and
//!   [`Reasoner::inferred`] after inserting the input into a reasoner with
//!   those switches and reasoning;
//! - `<name>.diagnostics.tsv`: one line per diagnostic: the code, the
//!   severity, the message, then each term in N-Triples syntax, separated by
//!   tabs (N-Triples writes a tab inside a literal as `\t`);
//! - `<name>.summary`: `base inferred diagnostics` of the run.

use oxrdf::Triple;
use std::fmt::Write as _;
use std::path::PathBuf;
use turbowl::{Reasoner, Severity, Switches};
use turbowl_oracle::cases::minimal;

fn main() {
    let dir: PathBuf = std::env::args_os().nth(1).expect("usage: minimal_cases <dir>").into();
    let configurations = [("default", Switches::default()), ("complete", Switches::complete())];
    for (configuration, _) in configurations {
        std::fs::create_dir_all(dir.join(configuration)).expect("create the directory");
    }
    let write = |name: String, text: String| {
        std::fs::write(dir.join(&name), text).unwrap_or_else(|e| panic!("{name}: {e}"));
    };
    let mut names = String::new();
    for case in minimal() {
        let name = case.name;
        let input = case.input();
        write(format!("{name}.input.nt"), ntriples(input.triples().iter().cloned()));
        for (configuration, switches) in configurations {
            let mut r = Reasoner::with_switches(switches);
            r.insert(input.triples().iter().map(Triple::as_ref));
            let summary = r.reason();
            let mut diagnostics = String::new();
            for d in r.diagnostics() {
                let severity = match d.severity {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                };
                let _ = write!(diagnostics, "{}\t{severity}\t{}", d.code, d.message);
                for term in &d.terms {
                    let _ = write!(diagnostics, "\t{term}");
                }
                diagnostics.push('\n');
            }
            let path = |file: &str| format!("{configuration}/{name}.{file}");
            write(path("closure.nt"), ntriples(r.triples()));
            write(path("inferred.nt"), ntriples(r.inferred()));
            write(path("diagnostics.tsv"), diagnostics);
            write(
                path("summary"),
                format!("{} {} {}\n", summary.base, summary.inferred, summary.diagnostics),
            );
        }
        names.push_str(name);
        names.push('\n');
    }
    write("cases.txt".to_owned(), names);
}

/// Triples as N-Triples, one per line.
fn ntriples(triples: impl Iterator<Item = Triple>) -> String {
    let mut out = String::new();
    for t in triples {
        let _ = writeln!(out, "{t} .");
    }
    out
}
