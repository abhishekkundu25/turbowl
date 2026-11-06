//! Switches: named groups of rules a caller turns on or off (ADR 0005).
//!
//! Every rule of the W3C OWL 2 RL tables belongs to the always-on core or
//! to exactly one [`Switch`]; [`Switches`] is the set of those that are on.
//! The rule table (`rules/mod.rs`) registers each family with its switch,
//! and the reasoner runs only the families the switches leave on. The
//! groups, their defaults and the reasons for them are in the crate doc.

use std::fmt;

/// A named group of rules a caller turns on or off. The rules of each
/// group are listed by their W3C names (OWL 2 Profiles §4.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Switch {
    /// `owl:sameAs` makes terms equal: eq-sym, eq-trans, eq-rep-s,
    /// eq-rep-p, eq-rep-o. On by default.
    Equality,
    /// `owl:propertyChainAxiom`: prp-spo2. On by default.
    PropertyChains,
    /// The rules that find contradictions and ill-typed literals: cax-dw,
    /// cax-adc, cls-com, cls-nothing2, cls-maxc1, cls-maxqc1, cls-maxqc2,
    /// prp-irp, prp-asyp, prp-pdw, prp-adp, prp-npa1, prp-npa2, eq-diff1,
    /// eq-diff2, eq-diff3, dt-not-type. On by default.
    Checks,
    /// Keys and cardinalities that make individuals the same: prp-key,
    /// cls-maxc2, cls-maxqc3, cls-maxqc4. Off by default.
    Identity,
    /// The schema rules beyond the class hierarchy: scm-eqc2, scm-spo,
    /// scm-eqp1, scm-eqp2, scm-dom1, scm-dom2, scm-rng1, scm-rng2, scm-hv,
    /// scm-svf1, scm-svf2, scm-avf1, scm-avf2, scm-int, scm-uni. Off by
    /// default.
    Schema,
    /// Triples that hold of every term, class, property or literal:
    /// eq-ref, scm-cls, scm-op, scm-dp, prp-ap, dt-type1, dt-type2. Off by
    /// default.
    Trivial,
    /// Literals compared by value: dt-eq, dt-diff. Off by default.
    LiteralValues,
}

// `Switch::ALL` lists every variant once, in declaration order: checked
// when the crate compiles, since `Switches` keeps one bit per position.
const _: () = {
    let mut i = 0;
    while i < Switch::ALL.len() {
        assert!(Switch::ALL[i] as usize == i);
        i += 1;
    }
    assert!(Switch::ALL.len() == Switch::LiteralValues as usize + 1);
};

impl Switch {
    /// Every switch, in the order the CLI and Python list them.
    pub const ALL: [Switch; 7] = [
        Switch::Equality,
        Switch::PropertyChains,
        Switch::Checks,
        Switch::Identity,
        Switch::Schema,
        Switch::Trivial,
        Switch::LiteralValues,
    ];

    /// The switch's name, as the CLI and Python spell it: `equality`,
    /// `property-chains`, `checks`, `identity`, `schema`, `trivial`,
    /// `literal-values`.
    pub const fn name(self) -> &'static str {
        match self {
            Switch::Equality => "equality",
            Switch::PropertyChains => "property-chains",
            Switch::Checks => "checks",
            Switch::Identity => "identity",
            Switch::Schema => "schema",
            Switch::Trivial => "trivial",
            Switch::LiteralValues => "literal-values",
        }
    }

    /// The switch called `name` (spelled as [`Switch::name`] gives it), or
    /// `None` when no switch has that name.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.name() == name)
    }

    /// Whether the switch is on in [`Switches::default`]: the crate doc
    /// gives the reason for each.
    const fn on_by_default(self) -> bool {
        match self {
            // Version 1's behaviour.
            Switch::Equality | Switch::PropertyChains | Switch::Checks => true,
            // More than 5% slower on a scoreboard scenario (the phase 1
            // close-out, 2026-11-06).
            Switch::Identity | Switch::Schema => false,
            // Rarely implemented elsewhere, and costly.
            Switch::Trivial | Switch::LiteralValues => false,
        }
    }

    const fn bit(self) -> u8 {
        1 << self as u8
    }
}

/// The switches that are on. [`Switches::default`] is the default
/// configuration ([`Switch::Equality`], [`Switch::PropertyChains`] and
/// [`Switch::Checks`] on); [`Switches::complete`] turns every switch on,
/// which is the W3C rule set.
///
/// ```
/// use turbowl::{Reasoner, Switch, Switches};
/// let fast = Switches::default().off(Switch::Equality).off(Switch::Checks);
/// assert!(!fast.is_on(Switch::Checks) && fast.is_on(Switch::PropertyChains));
/// let r = Reasoner::with_switches(fast);
/// assert_eq!(r.switches(), fast);
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Switches {
    /// Bit `s as u8` is set when switch `s` is on.
    on: u8,
}

impl Default for Switches {
    /// The default configuration: [`Switch::Equality`],
    /// [`Switch::PropertyChains`] and [`Switch::Checks`] on.
    fn default() -> Self {
        let mut switches = Self { on: 0 };
        for s in Switch::ALL {
            if s.on_by_default() {
                switches = switches.on(s);
            }
        }
        switches
    }
}

impl Switches {
    /// Every switch on: the complete W3C OWL 2 RL rule set.
    pub const fn complete() -> Self {
        let mut on = 0;
        let mut i = 0;
        while i < Switch::ALL.len() {
            on |= Switch::ALL[i].bit();
            i += 1;
        }
        Self { on }
    }

    /// These switches, with `switch` on.
    #[must_use]
    pub const fn on(self, switch: Switch) -> Self {
        Self {
            on: self.on | switch.bit(),
        }
    }

    /// These switches, with `switch` off.
    #[must_use]
    pub const fn off(self, switch: Switch) -> Self {
        Self {
            on: self.on & !switch.bit(),
        }
    }

    /// True when `switch` is on.
    pub const fn is_on(self, switch: Switch) -> bool {
        self.on & switch.bit() != 0
    }
}

/// The names of the switches that are on: `{"equality", "checks"}`.
impl fmt::Debug for Switches {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set()
            .entries(Switch::ALL.into_iter().filter(|&s| self.is_on(s)).map(Switch::name))
            .finish()
    }
}

#[cfg(all(test, feature = "io"))]
mod tests;
