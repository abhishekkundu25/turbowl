//! The W3C tests: the OWL 2 conformance tests in the RL profile and the RDF
//! 1.1 Semantics entailment tests, run on every oracle. Reports go to
//! `target/oracle/w3c-*.md`.
//!
//! Nemo runs exactly the version 1 rule set, so every case Nemo passes is
//! required of turtle-reasoner. The cases Nemo fails are listed with the
//! rules they likely need (`w3c::missing_rules`). The oracles' own verdicts
//! are reported, not asserted.

mod support;

use std::fmt::Write as _;
use turtle_reasoner_oracle::w3c::{self, Cases, Expect, Verdict};
use turtle_reasoner_oracle::{Outcome, nemo, ours, owlrl};

fn verdict(case: &w3c::Case, outcome: &Outcome) -> String {
    match outcome {
        Outcome::Ran(c) => match w3c::judge(case, c) {
            Verdict::Pass => "pass".to_owned(),
            Verdict::Fail(_) => "FAIL".to_owned(),
        },
        other => other.status().to_owned(),
    }
}

fn run(suite: &str, loaded: Result<Cases, Outcome>) {
    let cases = match loaded {
        Ok(c) => c,
        Err(Outcome::Skipped(why)) => {
            println!("{suite}: skipped: {why}");
            return;
        }
        Err(o) => panic!("{suite}: {o:?}"),
    };
    let oracles = ["Nemo", "OWL-RL", "ours"];
    let mut out = format!(
        "| Case | Expects | Approved | Regime | {} |\n|---|---|---|---|",
        oracles.join(" | ")
    );
    out.push_str(&"---|".repeat(oracles.len()));
    out.push('\n');
    let mut passes = [0usize; 3];
    let mut required = 0;
    let mut rest = Vec::new();
    let mut failures = Vec::new();
    for case in &cases.cases {
        let outcomes = [
            nemo::run(&case.premise),
            owlrl::run(&case.premise),
            ours::run(&case.premise),
        ];
        if let Outcome::Failed(why) = &outcomes[0] {
            panic!("{}: Nemo failed: {why}", case.id);
        }
        let verdicts: Vec<String> = outcomes.iter().map(|o| verdict(case, o)).collect();
        for (i, v) in verdicts.iter().enumerate() {
            passes[i] += usize::from(v == "pass");
        }
        // What the version 1 rule set (Nemo) gets right, turtle-reasoner must get right too.
        if verdicts[0] == "pass" {
            required += 1;
            if verdicts[2] == "FAIL" {
                let why = match w3c::judge(case, outcomes[2].closure().expect("ran")) {
                    Verdict::Fail(why) => why,
                    Verdict::Pass => unreachable!(),
                };
                failures.push(format!("{}: {why}", case.id));
            }
        } else {
            let missing = w3c::missing_rules(case);
            let missing = if missing.is_empty() {
                "unknown".to_owned()
            } else {
                missing.join("; ")
            };
            rest.push(format!("- {} (OWL-RL: {}): {missing}", case.id, verdicts[1]));
        }
        let expects = match case.expect {
            Expect::Entails(_) => "entails",
            Expect::NotEntails(_) => "not entails",
            Expect::Inconsistent => "inconsistent",
            Expect::Consistent => "consistent",
        };
        let _ = writeln!(
            out,
            "| {} | {expects} | {} | {} | {} |",
            case.id,
            if case.approved { "yes" } else { "no" },
            case.regime,
            verdicts.join(" | ")
        );
    }
    let _ = write!(out, "\nPassed, of {} runnable cases:", cases.cases.len());
    for (o, n) in oracles.iter().zip(passes) {
        let _ = write!(out, " {o} {n};");
    }
    let _ = writeln!(
        out,
        "\n\nThe version 1 rule set (Nemo) passes {required}; turtle-reasoner must pass each of them."
    );
    let _ = writeln!(
        out,
        "\nNemo fails {}, with the rules they likely need (from their vocabulary):",
        rest.len()
    );
    for r in &rest {
        let _ = writeln!(out, "{r}");
    }
    if !cases.unrunnable.is_empty() {
        let _ = writeln!(out, "\nNot runnable here ({}):", cases.unrunnable.len());
        for (id, why) in &cases.unrunnable {
            let _ = writeln!(out, "- {id}: {why}");
        }
    }
    let path = support::report(suite, &out);
    println!("{out}\n(written to {})", path.display());
    assert!(
        failures.is_empty(),
        "turtle-reasoner fails W3C cases the version 1 rule set passes:\n{}",
        failures.join("\n")
    );
}

#[test]
fn owl2_rl_conformance_tests() {
    run("w3c-owl2-rl", w3c::owl2_rl());
}

#[test]
fn rdf_semantics_entailment_tests() {
    run("w3c-rdf-mt", w3c::rdf_mt());
}
