//! TurbOWL itself, through its public interface: the system the
//! oracles judge.

use crate::{Closure, Finding, Input, Outcome};
use oxrdf::{NamedNodeRef, Term, Triple};
use turbowl::Reasoner;

/// True while the equality rules (eq-sym, eq-trans, eq-rep-s/p/o; ticket
/// 11) are not in the rule table. Ticket 11 sets it to false, and then every
/// input is compared.
pub const EQUALITY_PENDING: bool = true;

/// Why [`run`] leaves an input out while [`EQUALITY_PENDING`].
pub const EQUALITY_REASON: &str = "needs the equality rules (ticket 11)";

/// The terms whose presence lets the eq-* rules fire: `owl:sameAs`, and
/// the two property types whose rules (prp-fp, prp-ifp) derive it.
pub const EQUALITY_TERMS: [NamedNodeRef<'static>; 3] = [
    NamedNodeRef::new_unchecked("http://www.w3.org/2002/07/owl#sameAs"),
    NamedNodeRef::new_unchecked("http://www.w3.org/2002/07/owl#FunctionalProperty"),
    NamedNodeRef::new_unchecked("http://www.w3.org/2002/07/owl#InverseFunctionalProperty"),
];

/// True when the closure of `triples` can depend on the eq-* rules: some
/// triple has one of [`EQUALITY_TERMS`] as its subject, predicate or object.
///
/// On any other input the eq-* rules derive nothing, so a closure computed
/// without them must equal Nemo's (and the reference evaluator's) exactly:
///
/// 1. Rules never create terms. Every term of a derived triple is a term of
///    the input, of a family's facts (`owl:Thing`, `owl:Nothing`,
///    `owl:Class`, `rdf:type`), or a constant in the rule's head.
/// 2. So a triple holding `owl:sameAs` can only arise from an input triple
///    that holds it, or from a rule with `owl:sameAs` in its head: eq-sym
///    and eq-trans (which need a `sameAs` premise), prp-fp and prp-ifp.
///    Those two need `p rdf:type owl:FunctionalProperty` (or
///    `owl:InverseFunctionalProperty`); no rule has either term in its head,
///    so by (1) that premise needs the term in the input.
/// 3. Every eq-* rule needs a `sameAs` premise.
/// 4. So on an input without the three terms no `sameAs` triple ever holds,
///    and no eq-* rule fires.
pub fn needs_equality(triples: &[Triple]) -> bool {
    let is = |t: &Term| matches!(t, Term::NamedNode(n) if EQUALITY_TERMS.contains(&n.as_ref()));
    triples
        .iter()
        .any(|t| EQUALITY_TERMS.contains(&t.predicate.as_ref()) || is(&t.subject.clone().into()) || is(&t.object))
}

/// Runs the reasoner on the input's triples (inserted as they are, so blank
/// nodes keep the labels `Input` gave them). While [`EQUALITY_PENDING`], an
/// input that [`needs_equality`] is `Pending` instead.
pub fn run(input: &Input) -> Outcome {
    if EQUALITY_PENDING && needs_equality(input.triples()) {
        return Outcome::Pending(EQUALITY_REASON.to_owned());
    }
    let mut r = Reasoner::new();
    r.insert(input.triples().iter().map(Triple::as_ref));
    r.reason();
    let findings: Vec<Finding> = r.diagnostics().iter().map(Finding::from_diagnostic).collect();
    Outcome::Ran(Closure::new(r.triples(), findings))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(turtle: &str) -> Input {
        Input::turtle(
            "t",
            &format!(
                "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
                 @prefix ex: <http://example.org/> .\n{turtle}"
            ),
        )
    }

    #[test]
    fn equality_terms_in_any_position_need_equality() {
        for turtle in [
            "ex:a owl:sameAs ex:b .",
            "ex:p a owl:FunctionalProperty .",
            "ex:p a owl:InverseFunctionalProperty .",
            "owl:sameAs ex:note ex:x .",
            "ex:p ex:like owl:sameAs .",
        ] {
            assert!(needs_equality(input(turtle).triples()), "{turtle}");
            assert!(matches!(run(&input(turtle)), Outcome::Pending(_)), "{turtle}");
        }
        for turtle in [
            "ex:a ex:p ex:b .",
            "ex:p a owl:TransitiveProperty .",
            "ex:a ex:label \"owl:sameAs\" .",
            "ex:a ex:p <http://www.w3.org/2002/07/owl#sameAsNot> .",
        ] {
            assert!(!needs_equality(input(turtle).triples()), "{turtle}");
            assert!(matches!(run(&input(turtle)), Outcome::Ran(_)), "{turtle}");
        }
    }

    #[test]
    fn the_terms_are_the_owl_iris() {
        let iris: Vec<&str> = EQUALITY_TERMS.iter().map(|n| n.as_str()).collect();
        assert_eq!(
            iris,
            [
                "http://www.w3.org/2002/07/owl#sameAs",
                "http://www.w3.org/2002/07/owl#FunctionalProperty",
                "http://www.w3.org/2002/07/owl#InverseFunctionalProperty",
            ]
        );
    }
}
