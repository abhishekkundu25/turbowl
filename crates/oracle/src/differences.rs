//! The documented differences from `reasonable` (D1 to D10, in
//! docs/specs/v1-parity.md), each a filter applied to both sides before a
//! closure is compared with `reasonable`'s.

use crate::{Closure, Input};
use oxrdf::vocab::rdf;
use oxrdf::{NamedNodeRef, Term};
use std::collections::{HashMap, HashSet};

const OWL_SAME_AS: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("http://www.w3.org/2002/07/owl#sameAs");
const OWL_IFP: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/2002/07/owl#InverseFunctionalProperty");

/// A documented difference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Difference {
    /// prp-ifp: `reasonable` makes subjects equal whenever they share the property.
    D1,
    /// cls-com: `reasonable` never emits `OWLRL.CLS_COM`.
    D2,
    /// Language tags dropped on `reasonable`'s output.
    D3,
    /// Blank-node labels shared across `reasonable`'s input files.
    D4,
    /// Lists with a shared tail merged by `reasonable`.
    D5,
    /// Literal-subject triples derived, then dropped with a log line.
    D6,
    /// A second `update_graph` with an unchanged graph re-runs everything.
    D7,
    /// Panics on I/O and parse errors; usage errors exit 2.
    D8,
    /// JSON diagnostics with unescaped newlines.
    D9,
    /// Python returns plain literals typed `xsd:string`.
    D10,
}

impl Difference {
    pub const ALL: [Self; 10] = [
        Self::D1,
        Self::D2,
        Self::D3,
        Self::D4,
        Self::D5,
        Self::D6,
        Self::D7,
        Self::D8,
        Self::D9,
        Self::D10,
    ];

    /// How the closure comparison handles it.
    pub fn handling(self) -> &'static str {
        match self {
            Self::D1 => "drops owl:sameAs triples, every triple and finding naming a merged term, on both sides",
            Self::D2 => "drops OWLRL.CLS_COM findings on both sides",
            Self::D3 => "none needed: closures are compared in memory, never through files",
            Self::D4 => "none needed: Input scopes blank nodes per document before any oracle sees them",
            Self::D5 => "the input is not compared with reasonable; OWL-RL and Nemo judge it",
            Self::D6 => "none needed: literal-subject triples are absent on both sides",
            Self::D7 => "none needed: closures only (CLI and Python tests cover the rest)",
            Self::D8 => "none: CLI exit-code tests (ticket 12)",
            Self::D9 => "none: CLI JSON tests parse before comparing (ticket 12)",
            Self::D10 => "none: Python tests compare rdflib terms (ticket 13)",
        }
    }

    /// True when the difference can show on this input (in either closure).
    pub fn applies(self, input: &Input, left: &Closure, right: &Closure) -> bool {
        match self {
            Self::D1 => [left, right]
                .iter()
                .any(|c| c.triples.iter().any(|t| t.object == OWL_IFP.into())),
            Self::D2 => [left, right]
                .iter()
                .any(|c| c.findings.iter().any(|f| f.code == "OWLRL.CLS_COM")),
            Self::D5 => has_shared_list_tail(input),
            _ => false,
        }
    }

    /// True when the input must not be compared with `reasonable` at all.
    pub fn excludes(self) -> bool {
        self == Self::D5
    }

    /// Removes from a closure what this difference makes differ. `merged` is
    /// the set of terms either side made `owl:sameAs` another term (for D1).
    fn filter(self, closure: &mut Closure, merged: &HashSet<Term>) {
        match self {
            Self::D1 => {
                closure.triples.retain(|t| {
                    t.predicate != OWL_SAME_AS
                        && !merged.contains(&t.subject.clone().into())
                        && !merged.contains(&t.object)
                });
                closure.findings.retain(|f| !f.terms.iter().any(|t| merged.contains(t)));
            }
            Self::D2 => closure.findings.retain(|f| f.code != "OWLRL.CLS_COM"),
            _ => {}
        }
    }
}

/// Applies every difference that shows on this input to both closures, and
/// returns them. When one of them [`Difference::excludes`] the input, the
/// closures are left as they are.
pub fn apply(input: &Input, left: &mut Closure, right: &mut Closure) -> Vec<Difference> {
    let applied: Vec<Difference> = Difference::ALL
        .into_iter()
        .filter(|d| d.applies(input, left, right))
        .collect();
    if applied.iter().any(|d| d.excludes()) {
        return applied;
    }
    let merged: HashSet<Term> = [&*left, &*right]
        .iter()
        .flat_map(|c| c.triples.iter())
        .filter(|t| t.predicate == OWL_SAME_AS && Term::from(t.subject.clone()) != t.object)
        .flat_map(|t| [t.subject.clone().into(), t.object.clone()])
        .collect();
    for d in &applied {
        d.filter(left, &merged);
        d.filter(right, &merged);
    }
    applied
}

/// True when a list cell is the `rdf:rest` of one cell and also referenced by
/// another triple (a second `rdf:rest`, or a list head such as
/// `owl:intersectionOf`): `reasonable`'s `DisjointSets` merges such lists.
fn has_shared_list_tail(input: &Input) -> bool {
    let rests: HashSet<&Term> = input
        .triples()
        .iter()
        .filter(|t| t.predicate == rdf::REST && t.object != rdf::NIL.into())
        .map(|t| &t.object)
        .collect();
    let mut refs: HashMap<&Term, usize> = HashMap::new();
    for t in input.triples() {
        if rests.contains(&t.object) {
            *refs.entry(&t.object).or_default() += 1;
        }
    }
    refs.values().any(|&n| n > 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_shared_list_tails() {
        let shared = Input::turtle(
            "shared",
            "@prefix ex: <http://example.org/> . @prefix owl: <http://www.w3.org/2002/07/owl#> .
             ex:I owl:intersectionOf (ex:A ex:B) . ex:J owl:intersectionOf _:t .
             _:h <http://www.w3.org/1999/02/22-rdf-syntax-ns#rest> _:t .",
        );
        // `(ex:A ex:B)` has its own cells; only _:t is shared (rest of _:h, head of ex:J's list).
        assert!(has_shared_list_tail(&shared));
        let plain = Input::turtle(
            "plain",
            "@prefix ex: <http://example.org/> . @prefix owl: <http://www.w3.org/2002/07/owl#> .
             ex:I owl:intersectionOf (ex:A ex:B) . ex:J owl:intersectionOf (ex:A ex:B) .",
        );
        assert!(!has_shared_list_tail(&plain));
    }
}
