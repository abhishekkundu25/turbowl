//! cls-oo, in the core: `c owl:oneOf (x1 ... xn)` derives `x1 rdf:type c`,
//! ..., `xn rdf:type c` (OWL 2 Profiles, Table 6).
//!
//! Empty: version 2's ticket 08 fills this file. The class family calls
//! [`rules`] each round, after its other rules.

use crate::engine::Round;

/// One round of cls-oo. `ty` is `rdf:type` read through the round's canon.
pub(super) fn rules(_round: &mut Round<'_>, _ty: u32) {}

#[cfg(test)]
mod tests {
    use crate::rules::testing;

    /// Until ticket 08, an enumeration types none of its members.
    #[test]
    fn cls_oo_derives_nothing_yet() {
        let run = testing::run(&["class"], "ex:C owl:oneOf ( ex:a ex:b ) .");
        assert!(!run.has("ex:a rdf:type ex:C"));
        assert!(!run.has("ex:b rdf:type ex:C"));
    }
}
