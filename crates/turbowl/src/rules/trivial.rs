//! The trivial family (switch `trivial`, off by default): the rules whose
//! triples hold of every term, class, property or literal (OWL 2 Profiles,
//! Tables 4, 5, 8 and 9).
//!
//! - eq-ref: `x owl:sameAs x` for every term. An output rule: produced when
//!   the closure is read, never stored (the phase 1 spec, Implementation
//!   Decisions).
//! - scm-cls: for each class, `c rdfs:subClassOf c`, `c owl:equivalentClass
//!   c`, `c rdfs:subClassOf owl:Thing`, `owl:Nothing rdfs:subClassOf c`.
//! - scm-op and scm-dp: for each object and datatype property,
//!   `p rdfs:subPropertyOf p` and `p owl:equivalentProperty p`.
//! - prp-ap: each annotation property of OWL 2 (`rdfs:label`,
//!   `rdfs:comment`, `rdfs:seeAlso`, `rdfs:isDefinedBy`, `owl:deprecated`,
//!   `owl:versionInfo`, `owl:priorVersion`, `owl:backwardCompatibleWith`,
//!   `owl:incompatibleWith`) typed `owl:AnnotationProperty`: facts.
//! - dt-type1: each OWL 2 RL datatype typed `rdfs:Datatype`: facts.
//! - dt-type2: each literal typed with every datatype whose value space
//!   holds its value.
//!
//! Empty: version 2's ticket 08 fills this file, then ticket 10 adds
//! dt-type2.

use super::Context;
use crate::engine::Round;
use crate::term::TermId;

/// The heads of the group's empty-body rules, prp-ap and dt-type1 (ticket
/// 08). The reasoner inserts them only when the `trivial` switch is on.
pub(crate) const FACTS: &[[TermId; 3]] = &[];

/// One round of the trivial rules.
pub(crate) fn rules(_round: &mut Round<'_>, _cx: &mut Context<'_>) {}

#[cfg(test)]
mod tests {
    use crate::rules::testing;

    /// Until ticket 08 fills it, the family derives nothing and
    /// reports nothing, whatever the input uses.
    #[test]
    fn trivial_derives_nothing_yet() {
        let run = testing::run(&["trivial"], testing::EVERY_CONSTRUCT);
        assert!(run.inferred().is_empty(), "{:#?}", run.inferred());
        assert!(run.findings().is_empty(), "{:#?}", run.findings());
    }
}
