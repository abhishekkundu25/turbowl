//! The rules, one file per switch group (ADR 0005, and the table "The
//! switches" in `docs/specs/v2-1-scoreboard-and-rules.md`), as in
//! `oracles/rules/`: `core.rs`, then each group named after its switch
//! ([`Switch::name`], with `_` for `-`). The `checks` group has a second
//! file, `datatypes.rs`, for dt-not-type, which reads the datatype map
//! (`crate::datatype`).
//!
//! Each file holds one [`Family`]: its deriving rules, which
//! `crate::evaluate` runs every round over the closure's [`View`], and its
//! diagnostic rules, run once at the fixpoint. [`families`] says which files
//! a group has; a run evaluates the core and the groups its [`Switches`]
//! leave on ([`on`]), in [`Switch::ALL`] order. A rule ticket fills its
//! group's file without editing this one.

mod checks;
mod core;
mod datatypes;
mod equality;
mod identity;
mod literal_values;
mod property_chains;
mod schema;
mod trivial;

use crate::evaluate::{Round, View};
use oxrdf::Term;
use turbowl::{Switch, Switches};

/// The rules of one file: its deriving rules, run every round, and its
/// diagnostic rules, run once over the closure at the fixpoint.
pub(crate) struct Family {
    /// The deriving rules, in the order `derive` runs them.
    rules: &'static [&'static str],
    /// The diagnostic rules. Each name is also the code of what it finds.
    diagnostic_rules: &'static [&'static str],
    /// One round of the deriving rules.
    pub(crate) derive: fn(&View, &mut Round),
    /// The diagnostic rules over the closure: (code, terms), in the orders
    /// `crate::Diagnostics` documents.
    pub(crate) diagnose: fn(&View, &mut Vec<Found>),
}

/// A diagnostic as a rule finds it: its code and its terms.
pub(crate) type Found = (&'static str, Vec<Term>);

/// A family with no rules yet: the file of a group a rule ticket fills.
const EMPTY: Family = Family {
    rules: &[],
    diagnostic_rules: &[],
    derive: |_, _| {},
    diagnose: |_, _| {},
};

/// The files of a group (`None`: the core). A match, so a new switch has no
/// rules here until it has its file.
fn families(group: Option<Switch>) -> &'static [Family] {
    match group {
        None => &[self::core::FAMILY],
        Some(Switch::Equality) => &[equality::FAMILY],
        Some(Switch::PropertyChains) => &[property_chains::FAMILY],
        Some(Switch::Checks) => &[checks::FAMILY, datatypes::FAMILY],
        Some(Switch::Identity) => &[identity::FAMILY],
        Some(Switch::Schema) => &[schema::FAMILY],
        Some(Switch::Trivial) => &[trivial::FAMILY],
        Some(Switch::LiteralValues) => &[literal_values::FAMILY],
    }
}

/// The families `switches` leave on, in evaluation order: the core, then
/// each switch that is on, in [`Switch::ALL`] order.
pub(crate) fn on(switches: Switches) -> impl Iterator<Item = &'static Family> {
    let groups = Switch::ALL.into_iter().filter(move |&s| switches.is_on(s)).map(Some);
    std::iter::once(None).chain(groups).flat_map(families)
}

/// The deriving rules `switches` leave on, in the order a round runs them.
/// A triple two rules derive in the same round is counted for the first.
pub fn rules(switches: Switches) -> Vec<&'static str> {
    on(switches).flat_map(|f| f.rules.iter().copied()).collect()
}

/// The diagnostic rules `switches` leave on, run once over the closure at
/// the fixpoint. Each name is also the code of what it finds.
pub fn diagnostic_rules(switches: Switches) -> Vec<&'static str> {
    on(switches).flat_map(|f| f.diagnostic_rules.iter().copied()).collect()
}
