//! turtle-reasoner itself, through its public interface: the system the
//! oracles judge.

use crate::{Closure, Finding, Input, Outcome};
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::OnceLock;
use turtle_reasoner::Reasoner;

/// Runs the reasoner on the input's triples (inserted as they are, so blank
/// nodes keep the labels `Input` gave them). While the `Reasoner` is the
/// stub, returns `Pending`; any other panic propagates.
pub fn run(input: &Input) -> Outcome {
    if let Some(reason) = stub() {
        return Outcome::Pending(reason.clone());
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        let mut r = Reasoner::new();
        r.insert(input.triples().iter().map(|t| t.as_ref()));
        r.reason();
        let findings: Vec<Finding> = r.diagnostics().iter().map(Finding::from_diagnostic).collect();
        Closure::new(r.triples(), findings)
    }));
    match result {
        Ok(c) => Outcome::Ran(c),
        Err(payload) => match unimplemented_message(&*payload) {
            Some(m) => Outcome::Pending(m),
            None => resume_unwind(payload),
        },
    }
}

/// `Some(message)` while the `Reasoner` is the stub (probed once per process;
/// the probe's panic message is printed once).
fn stub() -> Option<&'static String> {
    static PROBE: OnceLock<Option<String>> = OnceLock::new();
    PROBE
        .get_or_init(|| {
            catch_unwind(|| {
                let mut r = Reasoner::new();
                r.reason();
                r.diagnostics().len()
            })
            .err()
            .and_then(|p| unimplemented_message(&*p))
        })
        .as_ref()
}

fn unimplemented_message(payload: &(dyn std::any::Any + Send)) -> Option<String> {
    let text = payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&str>().copied())?;
    text.starts_with("not implemented")
        .then(|| format!("turtle-reasoner: the Reasoner is the stub ({text})"))
}
