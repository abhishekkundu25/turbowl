//! Diagnostics: findings about the data (ticket 09).

use oxrdf::Term;

/// How serious a diagnostic is. `reasonable` reports every one as an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    Error,
    Warning,
}

/// A finding about the data: an inconsistency or an ill-formed literal.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Diagnostic {
    /// `OWLRL.CAX_DW`, `RDFS.DATATYPE`, ...
    pub code: &'static str,
    /// The rule: `cax-dw`, `rdfs-datatype`, ...
    pub rule: &'static str,
    pub severity: Severity,
    /// `reasonable`'s message text.
    pub message: String,
    /// The terms involved, in the rule's order.
    pub terms: Vec<Term>,
}
