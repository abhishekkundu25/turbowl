//! The rule table: every rule family registers here, one line each.
//!
//! A family is a module (`class`, `property`, `datatype`, `equality`) that,
//! given the engine's relations for a round, derives new tuples and
//! diagnostics. The function a family provides is defined with the engine
//! (ticket 03); this table is the only place the reasoner learns which
//! families exist.

/// One rule family.
#[allow(dead_code)] // used once ticket 10 runs the table; remove the allow then
#[derive(Debug)]
pub(crate) struct Family {
    pub(crate) name: &'static str,
}

/// Every family, in evaluation order. One line per family.
#[allow(dead_code)] // used once ticket 10 runs the table; remove the allow then
pub(crate) static FAMILIES: &[Family] = &[
    // class (ticket 07)
    // property (ticket 08)
    // datatype and diagnostics (ticket 09)
    // equality (ticket 11)
];
