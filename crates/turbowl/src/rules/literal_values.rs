//! The literal-values family (switch `literal-values`, off by default):
//! literals compared by value (OWL 2 Profiles, Table 8).
//!
//! - dt-eq: literals with equal values (in the OWL 2 value spaces, where an
//!   `xsd:integer` value is an `xsd:decimal` value) join one class in the
//!   canon, as `owl:sameAs` would make them.
//! - dt-diff: two literals with different values are different. Never
//!   listed: two such literals made `owl:sameAs` give the eq-diff1 finding,
//!   and the `owl:differentFrom` triples between all literals are not
//!   materialised.
//!
//! Empty: version 2's ticket 10 fills this file.

use super::Context;
use crate::engine::Round;
use crate::term::TermId;

/// The literal-value rules have no empty-body rule.
pub(crate) const FACTS: &[[TermId; 3]] = &[];

/// One round of the literal-values rules.
pub(crate) fn rules(_round: &mut Round<'_>, _cx: &mut Context<'_>) {}

#[cfg(test)]
mod tests {
    use crate::rules::testing;

    /// Until ticket 10 fills it, the family derives nothing and
    /// reports nothing, whatever the input uses.
    #[test]
    fn literal_values_derives_nothing_yet() {
        let run = testing::run(&["literal-values"], testing::EVERY_CONSTRUCT);
        assert!(run.inferred().is_empty(), "{:#?}", run.inferred());
        assert!(run.findings().is_empty(), "{:#?}", run.findings());
    }
}
