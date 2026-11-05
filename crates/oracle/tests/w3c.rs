//! The W3C tests: the OWL 2 conformance tests in the RL profile and the RDF
//! 1.1 Semantics entailment tests, run on every oracle in the complete
//! configuration (every switch on: the W3C rule set, ADR 0005). Reports go
//! to `target/oracle/w3c-*.md`.
//!
//! Nemo runs exactly the rules of the configuration, so every case Nemo
//! passes is required of TurbOWL. The cases Nemo fails are listed with the
//! rules they likely need that the program does not hold yet
//! (`w3c::missing_rules`). The oracles' own verdicts are reported, not
//! asserted.

mod support;

use std::fmt::Write as _;
use turbowl::Switches;
use turbowl_oracle::w3c::{self, Cases, Expect, Verdict};
use turbowl_oracle::{Outcome, nemo, ours, owlrl};

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
    let switches = Switches::complete();
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
            nemo::run_with(&case.premise, switches),
            owlrl::run(&case.premise),
            ours::run(&case.premise, switches),
        ];
        if let Outcome::Failed(why) = &outcomes[0] {
            panic!("{}: Nemo failed: {why}", case.id);
        }
        let verdicts: Vec<String> = outcomes.iter().map(|o| verdict(case, o)).collect();
        for (i, v) in verdicts.iter().enumerate() {
            passes[i] += usize::from(v == "pass");
        }
        // What the rules of the configuration (Nemo) get right, TurbOWL must get right too.
        if verdicts[0] == "pass" {
            required += 1;
            match &outcomes[2] {
                Outcome::Ran(c) => {
                    if let Verdict::Fail(why) = w3c::judge(case, c) {
                        failures.push(format!("{}: {why}", case.id));
                    }
                }
                other => failures.push(format!("{}: TurbOWL {}", case.id, other.status())),
            }
        } else {
            let missing = w3c::missing_rules(case, switches);
            let missing = if missing.is_empty() {
                "unknown".to_owned()
            } else {
                missing.join(", ")
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
        "\n\nIn the complete configuration, the rule program (Nemo) passes {required}; TurbOWL must pass each of them."
    );
    let _ = writeln!(
        out,
        "\nNemo fails {}, with the rules they likely need that the program does not hold yet (from their vocabulary):",
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
        "TurbOWL fails W3C cases the rule program of the configuration passes:\n{}",
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
