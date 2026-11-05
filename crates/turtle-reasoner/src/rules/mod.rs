//! The rule table: every rule family registers here, one line each.
//!
//! A family is a module (`class`, `property`, `datatype`, later `equality`)
//! with a [`RuleFn`] that runs one round of its rules, and the facts its
//! rules with an empty body state. This table is the only place the
//! reasoner learns which families exist.
//!
//! # How a family runs
//!
//! Each round the reasoner (ticket 10) calls every family's function, in
//! table order, with the same [`Round`] and [`Context`]:
//!
//! - The [`Round`] is the store as the last round left it, read-only, and
//!   the buffer for derived triples ([`Round::emit`]). A family joins the
//!   recent parts of the predicate tables with the rest (semi-naive), so
//!   every derivation is made once over a run. See `engine/mod.rs`.
//! - The [`Context`] holds the dictionary (a rule that looks inside a
//!   literal reads it) and the run's findings: a rule that concludes
//!   `false` (cax-dw, dt-not-type, ...) pushes a [`Finding`]. Findings never
//!   feed other rules; the reasoner deduplicates them after the run.
//!
//! The rules work on generalized triples (`docs/specs/v1.md`, Implementation
//! Decisions): any term id in any position, a literal included. A family
//! never filters a derived triple because its subject is a literal or its
//! predicate is not an IRI; only the reasoner's output leaves those out.
//!
//! A family keeps no state between calls (`fn` pointers in a `static`):
//! everything a run knows is in the store, so an incremental run (more
//! inserts, then run again) sees all of it.
//!
//! [`Family::facts`] are the heads of the rules with an empty body
//! (cls-thing, cls-nothing1). The reasoner inserts them at the start of
//! every full run, as inferred triples, so they hold even for an empty base.

// Ticket 10 runs the table; until then only the tests read it.
#![allow(dead_code)]

pub(crate) mod class;
pub(crate) mod datatype;
pub(crate) mod property;
#[cfg(test)]
pub(crate) mod testing;

use crate::engine::Round;
use crate::term::{Dictionary, TermId};

/// One round of a family's rules.
pub(crate) type RuleFn = fn(&mut Round<'_>, &mut Context<'_>);

/// What a family reads and writes besides the store's tables.
#[derive(Debug)]
pub(crate) struct Context<'a> {
    /// The terms, for rules that look inside a literal (dt-not-type).
    pub(crate) terms: &'a Dictionary,
    /// The findings of this run so far. Duplicates are allowed: the
    /// reasoner deduplicates on (code, terms).
    pub(crate) findings: &'a mut Vec<Finding>,
}

impl Context<'_> {
    /// Reports that a rule concluded `false`.
    pub(crate) fn report(&mut self, code: &'static str, terms: &[u32]) {
        self.findings.push(Finding {
            code,
            terms: terms.to_vec(),
        });
    }
}

/// A rule that concluded `false`, by its W3C name, with its term ids in the
/// order the oracle crate's `Finding` documents for that code. The public
/// `Diagnostic` is made from it (`diagnostics.rs`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Finding {
    pub(crate) code: &'static str,
    pub(crate) terms: Vec<u32>,
}

/// One rule family.
#[derive(Debug)]
pub(crate) struct Family {
    pub(crate) name: &'static str,
    /// One round of the family's rules.
    pub(crate) rules: RuleFn,
    /// `(subject, predicate, object)` facts: the heads of the family's rules
    /// with an empty body.
    pub(crate) facts: &'static [[TermId; 3]],
}

/// Every family, in evaluation order. One line per family.
#[rustfmt::skip]
pub(crate) static FAMILIES: &[Family] = &[
    Family { name: "class", rules: class::rules, facts: class::FACTS }, // ticket 07
    Family { name: "property", rules: property::rules, facts: property::FACTS }, // ticket 08
    Family { name: "datatype", rules: datatype::rules, facts: datatype::FACTS }, // ticket 09
    // equality (ticket 11)
];
