//! The reference evaluator (`crates/reference`) against Nemo: both run the
//! rules in their W3C form, composed for a configuration, so their closures
//! and findings must be equal (dt-not-type aside: Nemo has no datatype
//! checks), in the default and the complete configuration. Inputs: every
//! minimal case, then generated inputs from seeds (`REFERENCE_SEEDS`,
//! default 150, sizes 1 to 48, as the chaos test draws them). A failure
//! names the seed and the configuration, and saves the input (N-Triples)
//! under `target/oracle/reference-<seed>.md`.

mod support;

use std::collections::HashSet;
use turbowl::Switches;
use turbowl_oracle::{Closure, Finding, Input, Outcome, Relation, cases, compare, configurations, nemo};
use turbowl_reference::{Rng, evaluate_within, generate};

/// Inputs whose closure grows past this are skipped (equality cliques
/// explode in an evaluator without representatives).
const MAX_CLOSURE: usize = 20_000;
const MAX_SIZE: usize = 48;

/// The reference's closure as a [`Closure`], with the findings Nemo
/// reports in the configuration, or `None` past [`MAX_CLOSURE`].
fn reference(input: &Input, switches: Switches) -> Option<Closure> {
    let base: HashSet<_> = input.triples().iter().cloned().collect();
    let e = evaluate_within(&base, switches, MAX_CLOSURE)?;
    let codes = nemo::codes(switches);
    let findings = e
        .diagnostics
        .iter()
        .filter(|(code, _)| codes.contains(&code.as_str()))
        .map(|(code, terms)| Finding::new(code.clone(), terms.clone()));
    Some(Closure::new(e.closure, findings))
}

/// `Err` with the diff when the two disagree in the configuration;
/// `Ok(false)` when not compared.
fn agree(input: &Input, switches: Switches) -> Result<bool, String> {
    let Some(reference) = reference(input, switches) else {
        return Ok(false);
    };
    let nemo = match nemo::run_with(input, switches) {
        Outcome::Ran(c) => c,
        Outcome::Skipped(why) => {
            println!("skipped: {why}");
            return Ok(false);
        }
        other => return Err(format!("Nemo: {other:?}")),
    };
    let diff = compare(("reference", &reference), ("Nemo", &nemo), Relation::Equal);
    if diff.holds() { Ok(true) } else { Err(diff.to_string()) }
}

#[test]
fn reference_equals_nemo_on_the_minimal_inputs() {
    let mut failures = Vec::new();
    for (configuration, switches) in configurations() {
        let mut compared = 0;
        for case in cases::minimal() {
            match agree(&case.input(), switches) {
                Ok(c) => compared += usize::from(c),
                Err(diff) => failures.push(format!("{} ({configuration}):\n{diff}", case.name)),
            }
        }
        println!("{configuration}: reference = Nemo on {compared} minimal inputs");
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn reference_equals_nemo_on_generated_inputs() {
    let seeds: u64 = std::env::var("REFERENCE_SEEDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(150);
    let mut failures = Vec::new();
    for (configuration, switches) in configurations() {
        let (mut compared, mut skipped) = (0, 0);
        for seed in 0..seeds {
            let mut rng = Rng::new(seed);
            let size = 1 + rng.below(MAX_SIZE);
            let triples = generate(rng.next_u64(), size);
            let input = Input::from_triples(format!("seed {seed}"), triples);
            match agree(&input, switches) {
                Ok(true) => compared += 1,
                Ok(false) => skipped += 1,
                Err(diff) => {
                    let path = support::report(&format!("reference-{seed}"), &input.to_ntriples());
                    failures.push(format!(
                        "seed {seed} ({configuration}; size {size}, input in {}):\n{diff}",
                        path.display()
                    ));
                }
            }
        }
        println!("{configuration}: reference = Nemo on {compared} generated inputs ({skipped} not compared)");
    }
    assert!(
        failures.is_empty(),
        "{} seeds differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// With `checks` off, neither reports a finding, and the closures stay
/// equal: on the minimal inputs of the checks group.
#[test]
fn reference_equals_nemo_with_checks_off() {
    let switches = Switches::default().off(turbowl::Switch::Checks);
    assert!(nemo::codes(switches).is_empty());
    let mut failures = Vec::new();
    for case in cases::minimal()
        .iter()
        .filter(|c| c.group == Some(turbowl::Switch::Checks))
    {
        if let Err(diff) = agree(&case.input(), switches) {
            failures.push(format!("{}:\n{diff}", case.name));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
