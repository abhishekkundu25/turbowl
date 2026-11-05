//! The diagnostic rules and datatypes (ticket 09): cax-dw, prp-pdw,
//! prp-asyp, prp-irp, cls-nothing2, cls-com, and dt-not-type for the checked
//! datatypes. Not written yet.

use super::Context;
use crate::engine::Round;
use crate::term::TermId;

/// The diagnostic rules have no empty-body rule.
pub(crate) const FACTS: &[[TermId; 3]] = &[];

/// One round of the diagnostic rules.
pub(crate) fn rules(round: &mut Round<'_>, cx: &mut Context<'_>) {
    let _ = (round, cx);
}
