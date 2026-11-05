//! The identity family (switch `identity`): the rules that make
//! individuals the same, prp-key (`owl:hasKey`), cls-maxc2
//! (`owl:maxCardinality 1`), cls-maxqc3 and cls-maxqc4
//! (`owl:maxQualifiedCardinality 1`, with `owl:onClass` a class or
//! `owl:Thing`; OWL 2 Profiles, Tables 5 and 6).
//!
//! Empty: version 2's ticket 06 fills this file. The rules derive
//! `owl:sameAs`; with the `equality` switch on, the equality family merges
//! what they make equal through the canon, like any `owl:sameAs` (ADR 0003).

use super::Context;
use crate::engine::Round;
use crate::term::TermId;

/// The identity rules have no empty-body rule.
pub(crate) const FACTS: &[[TermId; 3]] = &[];

/// One round of the identity rules.
pub(crate) fn rules(_round: &mut Round<'_>, _cx: &mut Context<'_>) {}

#[cfg(test)]
mod tests {
    use crate::rules::testing;

    /// Until ticket 06 fills it, the family derives nothing and
    /// reports nothing, whatever the input uses.
    #[test]
    fn identity_derives_nothing_yet() {
        let run = testing::run(&["identity"], testing::EVERY_CONSTRUCT);
        assert!(run.inferred().is_empty(), "{:#?}", run.inferred());
        assert!(run.findings().is_empty(), "{:#?}", run.findings());
    }
}
