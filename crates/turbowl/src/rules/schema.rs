//! The schema family (switch `schema`): the schema rules beyond the
//! class hierarchy (OWL 2 Profiles, Table 9), scm-eqc2, scm-spo, scm-eqp1,
//! scm-eqp2, scm-dom1, scm-dom2, scm-rng1, scm-rng2, scm-hv, scm-svf1,
//! scm-svf2, scm-avf1, scm-avf2, scm-int and scm-uni. scm-sco and scm-eqc1
//! are in the core's class family; scm-cls, scm-op and scm-dp in the
//! `trivial` family.
//!
//! Empty: version 2's ticket 07 fills this file.

use super::Context;
use crate::engine::Round;
use crate::term::TermId;

/// The schema rules have no empty-body rule.
pub(crate) const FACTS: &[[TermId; 3]] = &[];

/// One round of the schema rules.
pub(crate) fn rules(_round: &mut Round<'_>, _cx: &mut Context<'_>) {}

#[cfg(test)]
mod tests {
    use crate::rules::testing;

    /// Until ticket 07 fills it, the family derives nothing and
    /// reports nothing, whatever the input uses.
    #[test]
    fn schema_derives_nothing_yet() {
        let run = testing::run(&["schema"], testing::EVERY_CONSTRUCT);
        assert!(run.inferred().is_empty(), "{:#?}", run.inferred());
        assert!(run.findings().is_empty(), "{:#?}", run.findings());
    }
}
