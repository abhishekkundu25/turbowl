//! TurbOWL itself, through its public interface: the system the
//! oracles judge.

use crate::{Closure, Finding, Input, Outcome};
use oxrdf::Triple;
use turbowl::Reasoner;

/// Runs the reasoner on the input's triples (inserted as they are, so blank
/// nodes keep the labels `Input` gave them).
pub fn run(input: &Input) -> Outcome {
    let mut r = Reasoner::new();
    r.insert(input.triples().iter().map(Triple::as_ref));
    r.reason();
    let findings: Vec<Finding> = r.diagnostics().iter().map(Finding::from_diagnostic).collect();
    Outcome::Ran(Closure::new(r.triples(), findings))
}
