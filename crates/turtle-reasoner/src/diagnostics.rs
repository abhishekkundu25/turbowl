//! Diagnostics: findings about the data (ticket 09).

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
