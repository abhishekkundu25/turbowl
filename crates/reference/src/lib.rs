//! The reference evaluator, the seeded generator, and the chaos test's
//! helpers (ticket 05).
//!
//! [`evaluate`] runs the rules in the form the W3C OWL 2 RL rule tables
//! write them, as naive loops over a `HashSet` of triples to a fixpoint (ADR
//! 0004). The rules are split into the groups the switches turn on and off
//! (ADR 0005), one file per group under `src/rules/`, and
//! [`evaluate_within`] runs the core and the groups a [`turbowl::Switches`]
//! value leaves on; [`evaluate`] runs the default configuration. It shares
//! no rule code with the core (only the `Switches` type, to name a
//! configuration) and makes no attempt to be fast: it is the definition the
//! chaos test (`tests/chaos.rs`) holds the core's `Reasoner` to, in the same
//! configuration, and the oracle crate compares it with Nemo. `owl:sameAs`
//! is closed by the eq-* rules as written, with no representatives, so it
//! checks the core's rewriting (ADR 0003) from the outside. The rules run
//! over generalized triples (a literal may be a subject inside) and the
//! closure returned holds only RDF triples: see the `evaluate` module's
//! conventions. The datatype map, which dt-not-type, dt-type2, dt-eq and
//! dt-diff read, is its own module (`src/datatype.rs`); [`DataValue`] gives
//! the oracle crate the values for Nemo, which reads literals as opaque
//! terms.
//!
//! [`generate`] draws a small ontology and its data from a seed; its shapes
//! are split by group too, under `src/generate/`.
//!
//! ```
//! use std::collections::HashSet;
//! use turbowl::{Switch, Switches};
//! let input: HashSet<_> = turbowl_reference::generate(7, 30).into_iter().collect();
//! let (closure, _diagnostics) = turbowl_reference::evaluate(&input);
//! assert!(closure.is_superset(&input));
//! let no_checks = Switches::default().off(Switch::Checks);
//! let e = turbowl_reference::evaluate_within(&input, no_checks, usize::MAX).expect("no limit");
//! assert!(e.diagnostics.is_empty());
//! ```

mod datatype;
mod evaluate;
mod generate;
mod rules;
mod vocab;

pub use datatype::DataValue;
pub use evaluate::{Diagnostics, Evaluation, evaluate, evaluate_counted, evaluate_within};
pub use generate::{Rng, generate};
pub use rules::{diagnostic_rules, rules};

use oxrdf::Triple;

/// Triples as an N-Triples document, one per line, sorted so that the same
/// set always gives the same text.
pub fn to_ntriples<'a>(triples: impl IntoIterator<Item = &'a Triple>) -> String {
    let mut lines: Vec<String> = triples.into_iter().map(|t| format!("{t} .\n")).collect();
    lines.sort_unstable();
    lines.concat()
}
