//! The class family (ticket 07): cax-sco, cax-eqc1/2, cls-thing,
//! cls-nothing1, cls-int1/2, cls-uni, cls-svf1/2, cls-avf, cls-hv1/2,
//! scm-sco and scm-eqc1, in their W3C forms. Not written yet.

use super::Context;
use crate::engine::Round;
use crate::term::TermId;

/// The heads of cls-thing and cls-nothing1.
pub(crate) const FACTS: &[[TermId; 3]] = &[];

/// One round of the class rules.
pub(crate) fn rules(round: &mut Round<'_>, cx: &mut Context<'_>) {
    let _ = (round, cx);
}
