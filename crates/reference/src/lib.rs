//! The reference evaluator, the seeded generator, and the chaos test's
//! helpers (ticket 05).
//!
//! [`evaluate`] runs the version 1 rule set (`docs/specs/v1.md`, "The rule
//! set of version 1") in the form the W3C OWL 2 RL rule tables write it, as
//! naive loops over a `HashSet` of triples to a fixpoint (ADR 0004). It
//! shares no code with the core and makes no attempt to be fast: it is the
//! definition the chaos test (`tests/chaos.rs`) holds the core's `Reasoner`
//! to, and the oracle crate compares it with Nemo. `owl:sameAs` is closed
//! by the eq-* rules as written, with no representatives, so it checks the
//! core's rewriting (ADR 0003) from the outside. The rules run over
//! generalized triples (a literal may be a subject inside) and the closure
//! returned holds only RDF triples: see the `evaluate` module's conventions.
//!
//! [`generate`] draws a small ontology and its data from a seed.
//!
//! ```
//! use std::collections::HashSet;
//! let input: HashSet<_> = turtle_reasoner_reference::generate(7, 30).into_iter().collect();
//! let (closure, _diagnostics) = turtle_reasoner_reference::evaluate(&input);
//! assert!(closure.is_superset(&input));
//! ```

mod datatype;
mod evaluate;
mod generate;
mod vocab;

pub use evaluate::{DIAGNOSTIC_RULES, Diagnostics, Evaluation, RULES, evaluate, evaluate_counted, evaluate_within};
pub use generate::{Rng, generate};

use oxrdf::Triple;

/// Triples as an N-Triples document, one per line, sorted so that the same
/// set always gives the same text.
pub fn to_ntriples<'a>(triples: impl IntoIterator<Item = &'a Triple>) -> String {
    let mut lines: Vec<String> = triples.into_iter().map(|t| format!("{t} .\n")).collect();
    lines.sort_unstable();
    lines.concat()
}
