//! Triangulation: every oracle on one input, each checked against the case's
//! expectations and compared with the others.
//!
//! | Check | Relation | Filters |
//! |---|---|---|
//! | `<oracle>: expectations` | the case's lines ([`crate::cases`]) | none; a fault listed in [`KNOWN`] is reported as known |
//! | `Nemo ⊆ OWL-RL` | containment (OWL-RL implements more rules) | the OWL-RL faults ([`Fault`]) |
//! | `ours = Nemo` | equal closures and findings, dt-not-type aside | none |
//! | `ours ⊆ OWL-RL` | containment | the OWL-RL faults |
//! | `HermiT entails <oracle>'s subsumptions` | soundness of inferred class and property subsumptions | none |

use crate::cases::{Case, Kind};
use crate::compare::{Relation, compare};
use crate::differences::{Fault, apply, explains_failure};
use crate::{CODES, Closure, Input, Outcome, hermit, nemo, ours, owlrl};
use std::fmt::Write as _;

/// Expectation checks an oracle fault makes fail: (case, check, fault). A
/// check listed here is reported as `known`, not as a failure; the closure
/// comparisons need no entries, since the fault filters apply there (F1
/// shows only there: `scm-sco-cycle` writes its reflexive triple as `+V`).
pub const KNOWN: &[(&str, &str, Fault)] = &[
    ("cls-com", "OWL-RL: expectations", Fault::F2),
    ("dt-not-type", "OWL-RL: expectations", Fault::F3),
    ("chain-cyclic-list", "OWL-RL: expectations", Fault::F4),
    ("dt-not-type-range", "OWL-RL: expectations", Fault::F5),
];

/// The result of one check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Pass,
    Fail,
    /// A disagreement an oracle fault explains.
    Known(Fault),
    /// Not run: an oracle was skipped or pending, or not asked to run.
    NotRun(String),
}

/// One check on one input.
#[derive(Debug, Clone)]
pub struct Check {
    pub name: String,
    pub status: Status,
    /// The readable diff or the failed expectations.
    pub detail: String,
}

/// Every oracle on one input.
#[derive(Debug, Clone)]
pub struct Row {
    pub case: String,
    /// What the input covers (the rules of a minimal case, or the data set).
    pub covers: String,
    /// (oracle, outcome).
    pub outcomes: Vec<(&'static str, Outcome)>,
    /// The oracle faults whose filters applied on this input.
    pub faults: Vec<Fault>,
    pub checks: Vec<Check>,
}

impl Row {
    fn outcome(&self, oracle: &str) -> &Outcome {
        &self
            .outcomes
            .iter()
            .find(|(o, _)| *o == oracle)
            .expect("every oracle has an outcome")
            .1
    }
}

/// Rows for several inputs.
#[derive(Debug, Clone, Default)]
pub struct Table {
    pub rows: Vec<Row>,
}

/// The oracles that give closures, in table order, with the kind of closure.
const ORACLES: &[(&str, Kind)] = &[
    ("Nemo", Kind::RuleSet),
    ("OWL-RL", Kind::FullRl),
    ("ours", Kind::RuleSet),
];

/// The finding codes an oracle covers.
fn codes(oracle: &str) -> &'static [&'static str] {
    match oracle {
        "Nemo" => nemo::CODES,
        "OWL-RL" => owlrl::CODES,
        _ => CODES,
    }
}

/// Which of the slower oracles run on an input.
#[derive(Debug, Clone, Copy)]
pub struct Options {
    /// OWL-RL (minutes on Brick-sized inputs).
    pub owlrl: bool,
    /// HermiT's subsumption check.
    pub hermit: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            owlrl: true,
            hermit: true,
        }
    }
}

/// Runs every oracle on a minimal case and checks them.
pub fn triangulate(case: &Case) -> Row {
    triangulate_input(case.name, case.covers, &case.input(), Some(case), Options::default())
}

/// Runs every oracle on an input and compares them; with a case, also checks
/// its expectations.
pub fn triangulate_input(name: &str, covers: &str, input: &Input, case: Option<&Case>, options: Options) -> Row {
    // The external oracles run side by side; each is a separate process.
    let (nemo, full) = std::thread::scope(|s| {
        let n = s.spawn(|| nemo::run(input));
        let o = s.spawn(|| {
            if options.owlrl {
                owlrl::run(input)
            } else {
                Outcome::Skipped("OWL-RL: not run on this input (see Options)".to_owned())
            }
        });
        let join =
            |h: std::thread::ScopedJoinHandle<'_, Outcome>| h.join().unwrap_or_else(|p| std::panic::resume_unwind(p));
        (join(n), join(o))
    });
    let outcomes = vec![("Nemo", nemo), ("OWL-RL", full), ("ours", ours::run(input))];
    let mut row = Row {
        case: name.to_owned(),
        covers: covers.to_owned(),
        outcomes,
        faults: Vec::new(),
        checks: Vec::new(),
    };

    for &(oracle, kind) in ORACLES {
        let Some(case) = case else { break };
        let name = format!("{oracle}: expectations");
        let check = match row.outcome(oracle) {
            Outcome::Ran(c) => {
                let failed = case.check(c, kind, codes(oracle));
                finish(case.name, name, failed.is_empty(), failed.join("\n"))
            }
            Outcome::Failed(why) => failed(oracle, input, case.name, name, why),
            other => not_run(name, other.status()),
        };
        row.checks.push(check);
    }

    let pairs: &[(&str, &str, Relation)] = &[
        ("Nemo", "OWL-RL", Relation::Subset),
        ("ours", "Nemo", Relation::Equal),
        ("ours", "OWL-RL", Relation::Subset),
    ];
    for &(left, right, relation) in pairs {
        let sym = if relation == Relation::Equal { "=" } else { "⊆" };
        let name = format!("{left} {sym} {right}");
        let check = match (row.outcome(left), row.outcome(right)) {
            (Outcome::Ran(l), Outcome::Ran(r)) => {
                let shared: Vec<&str> = codes(left)
                    .iter()
                    .copied()
                    .filter(|c| codes(right).contains(c))
                    .collect();
                let (mut l, mut r) = (only_codes(l, &shared), only_codes(r, &shared));
                for f in apply(right, input, &mut l, &mut r) {
                    if !row.faults.contains(&f) {
                        row.faults.push(f);
                    }
                }
                let diff = compare((left, &l), (right, &r), relation);
                finish(&row.case, name, diff.holds(), diff.to_string())
            }
            (Outcome::Ran(_), Outcome::Failed(why)) => failed(right, input, &row.case, name, why),
            (Outcome::Failed(why), _) => failed(left, input, &row.case, name, why),
            (l, r) => not_run(name, &format!("{} / {}", l.status(), r.status())),
        };
        row.checks.push(check);
    }

    for oracle in ["Nemo", "ours"] {
        let name = format!("HermiT entails {oracle}'s subsumptions");
        let check = match row.outcome(oracle) {
            _ if !options.hermit => not_run(name, "not run on this input (see Options)"),
            Outcome::Ran(c) => match hermit::check(input, c) {
                Ok(s) if s.checked == 0 => not_run(name, "no inferred subsumption"),
                Ok(s) => finish(
                    &row.case,
                    name,
                    s.not_entailed.is_empty(),
                    format!(
                        "{} checked{}{}",
                        s.checked,
                        if s.inconsistent {
                            " (input inconsistent for HermiT)"
                        } else {
                            ""
                        },
                        s.not_entailed
                            .iter()
                            .map(|n| format!("\n  not entailed: {n}"))
                            .collect::<String>()
                    ),
                ),
                Err(o) => not_run(name, &o_reason(&o)),
            },
            other => not_run(name, other.status()),
        };
        row.checks.push(check);
    }
    row
}

fn o_reason(o: &Outcome) -> String {
    match o {
        Outcome::Skipped(r) | Outcome::Pending(r) | Outcome::Failed(r) => format!("{}: {r}", o.status()),
        Outcome::Ran(_) => "ran".to_owned(),
    }
}

fn only_codes(c: &Closure, codes: &[&str]) -> Closure {
    Closure {
        triples: c.triples.clone(),
        findings: c
            .findings
            .iter()
            .filter(|f| codes.contains(&f.code.as_str()))
            .cloned()
            .collect(),
    }
}

fn not_run(name: String, why: &str) -> Check {
    Check {
        name,
        status: Status::NotRun(why.to_owned()),
        detail: String::new(),
    }
}

/// A check whose oracle failed: known when a fault explains the failure.
fn failed(oracle: &str, input: &Input, case: &str, name: String, why: &str) -> Check {
    match explains_failure(oracle, input) {
        Some(f) => Check {
            name,
            status: Status::Known(f),
            detail: why.to_owned(),
        },
        None => finish(case, name, false, why.to_owned()),
    }
}

fn finish(case: &str, name: String, ok: bool, detail: String) -> Check {
    let status = if ok {
        Status::Pass
    } else if let Some((_, _, fault)) = KNOWN.iter().find(|(c, n, _)| *c == case && *n == name) {
        Status::Known(*fault)
    } else {
        Status::Fail
    };
    Check { name, status, detail }
}

impl Table {
    /// The failed checks: (case, check).
    pub fn failures(&self) -> Vec<(&str, &Check)> {
        self.rows
            .iter()
            .flat_map(|r| r.checks.iter().map(move |c| (r.case.as_str(), c)))
            .filter(|(_, c)| c.status == Status::Fail)
            .collect()
    }

    /// Entries of [`KNOWN`] whose check passed or did not run (stale entries).
    pub fn stale_known(&self) -> Vec<(&'static str, &'static str)> {
        KNOWN
            .iter()
            .filter(|(case, name, _)| {
                !self.rows.iter().any(|r| {
                    r.case == *case
                        && r.checks
                            .iter()
                            .any(|c| c.name == *name && matches!(c.status, Status::Known(_)))
                })
            })
            .map(|(c, n, _)| (*c, *n))
            .collect()
    }

    /// The results as Markdown: one row per input with each oracle's closure
    /// size and findings, then every check that did not pass, then the
    /// oracle faults that showed.
    pub fn markdown(&self) -> String {
        let mut out = String::new();
        out.push_str("| Case | Covers |");
        for (o, _) in ORACLES {
            let _ = write!(out, " {o} |");
        }
        out.push_str(" Checks |\n|---|---|");
        for _ in ORACLES {
            out.push_str("---|");
        }
        out.push_str("---|\n");
        for r in &self.rows {
            let _ = write!(out, "| {} | {} |", r.case, r.covers);
            for (o, _) in ORACLES {
                let cell = match r.outcome(o) {
                    Outcome::Ran(c) => format!("{} · {}", c.triples.len(), c.finding_summary()),
                    other => other.status().to_owned(),
                };
                let _ = write!(out, " {cell} |");
            }
            let count = |f: fn(&Status) -> bool| r.checks.iter().filter(|c| f(&c.status)).count();
            let mut faults: Vec<Fault> = r.faults.clone();
            for c in &r.checks {
                if let Status::Known(f) = c.status
                    && !faults.contains(&f)
                {
                    faults.push(f);
                }
            }
            faults.sort_by_key(|f| *f as u8);
            let _ = writeln!(
                out,
                " {} pass, {} fail, {} known, {} not run{} |",
                count(|s| *s == Status::Pass),
                count(|s| *s == Status::Fail),
                count(|s| matches!(s, Status::Known(_))),
                count(|s| matches!(s, Status::NotRun(_))),
                if faults.is_empty() {
                    String::new()
                } else {
                    format!(
                        "; {}",
                        faults.iter().map(|f| format!("{f:?}")).collect::<Vec<_>>().join(", ")
                    )
                }
            );
        }
        out.push_str(
            "\nCells: closure triples · findings. Nemo and ours run the version 1 rule set; OWL-RL the full \
             W3C rule set. Fn: an oracle fault (crates/oracle/src/differences.rs) filtered or known on this \
             input.\n",
        );
        for r in &self.rows {
            for c in r
                .checks
                .iter()
                .filter(|c| matches!(c.status, Status::Fail | Status::Known(_)))
            {
                let _ = writeln!(
                    out,
                    "\n### {}: {} ({})\n",
                    r.case,
                    c.name,
                    match &c.status {
                        Status::Known(f) => format!("known: {f:?}, {}", f.description()),
                        _ => "FAIL".to_owned(),
                    }
                );
                let _ = writeln!(out, "```\n{}\n```", c.detail.trim_end());
            }
        }
        let skipped: Vec<String> = self
            .rows
            .iter()
            .flat_map(|r| r.outcomes.iter())
            .filter(|(_, o)| !matches!(o, Outcome::Ran(_)))
            .map(|(_, o)| o_reason(o))
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        let not_run: std::collections::BTreeSet<String> = self
            .rows
            .iter()
            .flat_map(|r| r.checks.iter())
            .filter_map(|c| match &c.status {
                Status::NotRun(why) if why.contains(':') => Some(format!("{}: {why}", c.name)),
                _ => None,
            })
            .collect();
        let skipped: Vec<String> = skipped.into_iter().chain(not_run).collect();
        if !skipped.is_empty() {
            out.push_str("\nNot run:\n");
            for s in skipped {
                let _ = writeln!(out, "- {s}");
            }
        }
        out
    }
}
