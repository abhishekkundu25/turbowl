//! Every inferred triple and every diagnostic of every minimal input
//! (`cases::minimal`), explained by the core and checked by the reference
//! rules (`turbowl_reference::check_proof`), in the default configuration
//! with `equality` off and the complete configuration with `equality` and
//! `literal-values` off (ticket 01 of `docs/tickets/explain/`). A missing
//! proof fails as a wrong one does.

use std::collections::HashSet;
use turbowl::{Goal, Reasoner, Switch, Switches};
use turbowl_oracle::cases;
use turbowl_reference::check_proof;

/// The configurations proofs are checked in.
fn configurations() -> [(&'static str, Switches); 2] {
    [
        ("default without equality", Switches::default().off(Switch::Equality)),
        (
            "complete without equality and literal values",
            Switches::complete().off(Switch::Equality).off(Switch::LiteralValues),
        ),
    ]
}

#[test]
fn every_inference_of_the_minimal_inputs_has_a_valid_proof() {
    let mut failures = Vec::new();
    for (configuration, switches) in configurations() {
        let (mut triples, mut diagnostics) = (0, 0);
        for case in cases::minimal() {
            let input = case.input();
            let base: HashSet<_> = input.triples().iter().cloned().collect();
            let mut r = Reasoner::with_switches(switches);
            r.insert(input.triples().iter().map(|t| t.as_ref()));
            r.reason();
            let explainer = r.explainer().expect("reasoned");
            let mut fail = |what: String| failures.push(format!("{} ({configuration}): {what}", case.name));
            for t in r.inferred() {
                match explainer.explain(t.as_ref()) {
                    None => fail(format!("no proof of {t}")),
                    Some(proof) => {
                        let goal = proof.goal();
                        if !matches!(goal, Goal::Statement(s) if s.subject == t.subject.clone().into()
                            && s.predicate == t.predicate.clone().into() && s.object == t.object)
                        {
                            fail(format!("the proof of {t} proves {goal:?}"));
                        }
                        if let Err(e) = check_proof(&proof, &base, switches) {
                            fail(format!("the proof of {t}: {e}"));
                        }
                        triples += 1;
                    }
                }
            }
            for d in r.diagnostics() {
                match explainer.explain_diagnostic(d) {
                    None => fail(format!("no proof of {} {:?}", d.code, d.terms)),
                    Some(proof) => {
                        if let Err(e) = check_proof(&proof, &base, switches) {
                            fail(format!("the proof of {} {:?}: {e}", d.code, d.terms));
                        }
                        diagnostics += 1;
                    }
                }
            }
        }
        println!("{configuration}: {triples} inferred triples and {diagnostics} diagnostics proved");
        assert!(triples > 0 && diagnostics > 0, "{configuration}: nothing explained");
    }
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
