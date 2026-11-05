//! The reference evaluator (`crates/reference`) against Nemo: both run the
//! version 1 rule set in its W3C form, so their closures and findings must be
//! equal (dt-not-type aside: Nemo has no datatype checks). Inputs: every
//! minimal case, then generated inputs from seeds (`REFERENCE_SEEDS`, default
//! 150, sizes 1 to 48, as the chaos test draws them). A failure names the
//! seed and saves the input (N-Triples) under `target/oracle/reference-<seed>.md`.

mod support;

use std::collections::HashSet;
use turtle_reasoner_oracle::{Closure, Finding, Input, Outcome, Relation, cases, compare, nemo};
use turtle_reasoner_reference::{Rng, evaluate_within, generate};

/// Inputs whose closure grows past this are skipped (equality cliques
/// explode in an evaluator without representatives).
const MAX_CLOSURE: usize = 20_000;
const MAX_SIZE: usize = 48;

/// The reference's closure as a [`Closure`], or `None` past [`MAX_CLOSURE`].
fn reference(input: &Input) -> Option<Closure> {
    let base: HashSet<_> = input.triples().iter().cloned().collect();
    let e = evaluate_within(&base, MAX_CLOSURE)?;
    let findings = e
        .diagnostics
        .iter()
        .filter(|(code, _)| nemo::CODES.contains(&code.as_str()))
        .map(|(code, terms)| Finding::new(code.clone(), terms.clone()));
    Some(Closure::new(e.closure, findings))
}

/// `Err` with the diff when the two disagree; `Ok(false)` when not compared.
fn agree(input: &Input) -> Result<bool, String> {
    let Some(reference) = reference(input) else {
        return Ok(false);
    };
    let nemo = match nemo::run(input) {
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
    let mut compared = 0;
    for case in cases::minimal() {
        match agree(&case.input()) {
            Ok(c) => compared += usize::from(c),
            Err(diff) => failures.push(format!("{}:\n{diff}", case.name)),
        }
    }
    println!("reference = Nemo on {compared} minimal inputs");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn reference_equals_nemo_on_generated_inputs() {
    let seeds: u64 = std::env::var("REFERENCE_SEEDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(150);
    let mut failures = Vec::new();
    let (mut compared, mut skipped) = (0, 0);
    for seed in 0..seeds {
        let mut rng = Rng::new(seed);
        let size = 1 + rng.below(MAX_SIZE);
        let triples = generate(rng.next_u64(), size);
        let input = Input::from_triples(format!("seed {seed}"), triples);
        match agree(&input) {
            Ok(true) => compared += 1,
            Ok(false) => skipped += 1,
            Err(diff) => {
                let path = support::report(&format!("reference-{seed}"), &input.to_ntriples());
                failures.push(format!(
                    "seed {seed} (size {size}, input in {}):\n{diff}",
                    path.display()
                ));
            }
        }
    }
    println!("reference = Nemo on {compared} generated inputs ({skipped} not compared)");
    assert!(
        failures.is_empty(),
        "{} seeds differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
