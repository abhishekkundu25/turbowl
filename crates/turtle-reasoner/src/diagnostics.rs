//! Diagnostics: findings about the data (ticket 09).

use crate::rules::Finding;
use crate::term::{Dictionary, TermId};
use oxrdf::Term;

/// How serious a diagnostic is. Every inconsistency of the rule set is an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    Error,
    Warning,
}

/// A finding about the data: an inconsistency or an ill-formed literal.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Diagnostic {
    /// The W3C rule that found it: `cax-dw`, `prp-irp`, `dt-not-type`, ...
    pub code: &'static str,
    pub severity: Severity,
    /// A sentence naming the terms.
    pub message: String,
    /// The terms involved, in the rule's order.
    pub terms: Vec<Term>,
}

impl Diagnostic {
    /// The public form of a rule's finding: its terms read from the
    /// dictionary. Ticket 09 writes the severity and the message.
    #[allow(dead_code)] // the reasoner (ticket 10) calls it
    pub(crate) fn from_finding(finding: &Finding, terms: &Dictionary) -> Self {
        Self {
            code: finding.code,
            severity: Severity::Error,
            message: finding.code.to_owned(),
            terms: finding.terms.iter().map(|&t| terms.to_oxrdf(TermId(t))).collect(),
        }
    }
}
