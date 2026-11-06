//! The rule table: every rule family registers here, one line each, with
//! its switch.
//!
//! A family is a module with a [`RuleFn`] that runs one round of its rules,
//! and the facts its rules with an empty body state. This table is the only
//! place the reasoner learns which families exist.
//!
//! # Families and switches
//!
//! Every family belongs to the always-on core (`switch: None`) or to one
//! [`Switch`] (ADR 0005); a switch may hold several families. The reasoner
//! runs the families the caller's [`Switches`] leave on ([`Family::is_on`]),
//! and inserts a family's facts only when it is on. The families follow the
//! groups of `docs/specs/v2-1-scoreboard-and-rules.md` ("The switches"):
//!
//! | Family | Switch | Rules |
//! |---|---|---|
//! | `class` | core | cax-sco, cax-eqc1/2, cls-thing, cls-nothing1, cls-int1/2, cls-uni, cls-svf1/2, cls-avf, cls-hv1/2, scm-sco, scm-eqc1; cls-oo (`class/enumerations.rs`) |
//! | `property` | core | prp-dom, prp-rng, prp-fp, prp-ifp, prp-symp, prp-trp, prp-spo1, prp-eqp1/2, prp-inv1/2 |
//! | `property-chains` | `property-chains` | prp-spo2 (`property/chain.rs`) |
//! | `checks` | `checks` | the rules that conclude `false`, dt-not-type aside |
//! | `datatype` | `checks` | dt-not-type |
//! | `equality` | `equality` | eq-sym, eq-trans, eq-rep-s/p/o |
//! | `identity` | `identity` | prp-key, cls-maxc2, cls-maxqc3/4 |
//! | `schema` | `schema` | scm-eqc2, scm-spo, scm-eqp1/2, scm-dom1/2, scm-rng1/2, scm-hv, scm-svf1/2, scm-avf1/2, scm-int, scm-uni |
//! | `trivial` | `trivial` | eq-ref, scm-cls, scm-op, scm-dp, prp-ap, dt-type1, dt-type2 |
//! | `literal-values` | `literal-values` | dt-eq, dt-diff |
//!
//! A rule ticket fills its family's file (its rule function and its
//! [`Family::facts`]) without editing this table; the vocabulary
//! (`term.rs`) already names every IRI the W3C rules name.
//!
//! # How a family runs
//!
//! Each round the reasoner calls every family that is on, in table order,
//! with a [`Round`] and a [`Context`]. In a round large enough to run in
//! parallel (ticket 17), each family is a task with a round and findings of
//! its own ([`Round::tasks`]), joined in table order afterwards, so a family
//! sees only the store and its own output, never another family's:
//!
//! - The [`Round`] is the store as the last round left it, read-only, and
//!   the buffer for derived triples ([`Round::emit`]). A family joins the
//!   recent parts of the predicate tables with the rest (semi-naive), so
//!   every derivation is made once over a run. See `engine/mod.rs`.
//! - The [`Context`] holds the dictionary (a rule that looks inside a
//!   literal reads it), the run's switches (a rule whose premises another
//!   switch's rules give reads them: the checks read eq-ref's
//!   `x owl:sameAs x` when `trivial` is on) and the run's findings: a rule
//!   that concludes `false` (cax-dw, dt-not-type, ...) pushes a
//!   [`Finding`]. Findings never feed other rules; the reasoner
//!   deduplicates them after the run.
//!
//! The rules work on generalized triples (`docs/specs/v1.md`, Implementation
//! Decisions): any term id in any position, a literal included. A family
//! never filters a derived triple because its subject is a literal or its
//! predicate is not an IRI; only the reasoner's output leaves those out.
//!
//! The tuples a family reads are in representatives (ADR 0003): the
//! equality family merges the terms `owl:sameAs` makes equal, and the store
//! rewrites its tuples. A family reads every constant it names through the
//! round ([`Round::constant`]), never as a bare `vocab` id, so that a
//! constant made equal to another term keeps its meaning.
//!
//! A family keeps no state between calls (`fn` pointers in a `static`):
//! everything a run knows is in the store, so an incremental run (more
//! inserts, then run again) sees all of it.
//!
//! A family may read a large recent part in chunks ([`Round::split`],
//! [`Round::split_runs`], [`Round::join`]): in a parallel round each chunk
//! runs as a task with a round of its own. What a family emits must not
//! depend on where the chunks are cut, and it asks for merges only in an
//! order that does not ([`Round::merge`]): the equality family reads the
//! `owl:sameAs` delta whole, in order.
//!
//! [`Family::facts`] are the heads of the rules with an empty body
//! (cls-thing, cls-nothing1). The reasoner inserts them at the start of
//! every full run, as inferred triples, so they hold even for an empty base.

pub(crate) mod checks;
pub(crate) mod class;
pub(crate) mod datatype;
pub(crate) mod equality;
pub(crate) mod identity;
pub(crate) mod literal_values;
pub(crate) mod property;
pub(crate) mod schema;
#[cfg(test)]
pub(crate) mod testing;
pub(crate) mod trivial;

use crate::engine::Round;
use crate::switch::{Switch, Switches};
use crate::term::{Dictionary, TermId};

/// One round of a family's rules.
pub(crate) type RuleFn = fn(&mut Round<'_>, &mut Context<'_>);

/// What a family reads and writes besides the store's tables.
#[derive(Debug)]
pub(crate) struct Context<'a> {
    /// The terms, for rules that look inside a literal (dt-not-type).
    pub(crate) terms: &'a Dictionary,
    /// The run's switches. A family runs only when its own switch is on;
    /// it reads them for what the families of other switches would give
    /// (eq-diff1, eq-diff2 and eq-diff3 read eq-ref's `x owl:sameAs x`,
    /// which is never stored, when `trivial` is on).
    pub(crate) switches: Switches,
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
    /// The switch that turns the family on and off; `None` for the core,
    /// which is always on.
    pub(crate) switch: Option<Switch>,
    /// One round of the family's rules.
    pub(crate) rules: RuleFn,
    /// `(subject, predicate, object)` facts: the heads of the family's rules
    /// with an empty body.
    pub(crate) facts: &'static [[TermId; 3]],
}

impl Family {
    /// True when `switches` leave the family on: it is in the core, or its
    /// switch is on.
    pub(crate) fn is_on(&self, switches: Switches) -> bool {
        self.switch.is_none_or(|s| switches.is_on(s))
    }
}

/// Every family, in evaluation order. One line per family.
#[rustfmt::skip]
pub(crate) static FAMILIES: &[Family] = &[
    Family { name: "class", switch: None, rules: class::rules, facts: class::FACTS },
    Family { name: "property", switch: None, rules: property::rules, facts: property::FACTS },
    Family { name: "property-chains", switch: Some(Switch::PropertyChains), rules: property::chain::rules, facts: property::chain::FACTS },
    Family { name: "checks", switch: Some(Switch::Checks), rules: checks::rules, facts: checks::FACTS },
    Family { name: "datatype", switch: Some(Switch::Checks), rules: datatype::rules, facts: datatype::FACTS },
    Family { name: "equality", switch: Some(Switch::Equality), rules: equality::rules, facts: equality::FACTS },
    Family { name: "identity", switch: Some(Switch::Identity), rules: identity::rules, facts: identity::FACTS },
    Family { name: "schema", switch: Some(Switch::Schema), rules: schema::rules, facts: schema::FACTS },
    Family { name: "trivial", switch: Some(Switch::Trivial), rules: trivial::rules, facts: trivial::FACTS },
    Family { name: "literal-values", switch: Some(Switch::LiteralValues), rules: literal_values::rules, facts: literal_values::FACTS },
];
