//! The property family (ticket 08): prp-dom, prp-rng, prp-fp, prp-ifp,
//! prp-symp, prp-trp, prp-spo1, prp-spo2, prp-eqp1/2 and prp-inv1/2, in
//! their W3C forms. Not written yet.

use super::Context;
use crate::engine::Round;
use crate::term::TermId;

/// The property rules have no empty-body rule.
pub(crate) const FACTS: &[[TermId; 3]] = &[];

/// One round of the property rules.
pub(crate) fn rules(round: &mut Round<'_>, cx: &mut Context<'_>) {
    let _ = (round, cx);
}
