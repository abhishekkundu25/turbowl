//! cls-oo, in the core: `c owl:oneOf (y1 ... yn)` derives `y1 rdf:type c`,
//! ..., `yn rdf:type c` (OWL 2 Profiles, Table 6).
//!
//! The list is read as cls-int2 and cls-uni read theirs (`lists.rs`): the
//! members are the `rdf:first` of the well-formed cells on the paths from
//! the head to `rdf:nil`, so a cyclic list, or one whose every path has a
//! cell without an `rdf:first`, types nothing. The rule needs no other
//! premise, so a list is read only in a round in which it is changed (its
//! `owl:oneOf` pair is recent, or one of its cells has a recent
//! `rdf:first` or `rdf:rest`); an unchanged list has typed its members
//! already.

use super::lists::Reader;
use crate::engine::{Pair, Round, Variable};
use crate::term::vocab;

/// One round of cls-oo. `ty` is `rdf:type` read through the round's canon.
pub(super) fn rules(round: &mut Round<'_>, ty: u32) {
    let Some(one_of) = round.by_subject(round.constant(vocab::OWL_ONE_OF.0)) else {
        return;
    };
    let first = round.by_subject(round.constant(vocab::RDF_FIRST.0));
    let rest = round.by_subject(round.constant(vocab::RDF_REST.0));
    let recent = |v: Option<&Variable<Pair>>| v.is_some_and(|v| !v.recent().is_empty());
    if one_of.recent().is_empty() && !recent(first) && !recent(rest) {
        return;
    }
    let nil = round.constant(vocab::RDF_NIL.0);
    let nil_is_a_cell = round.canon().members(nil).len() > 1;
    let lists = Reader::new(first, rest, nil, nil_is_a_cell).read(one_of);
    for (class, members) in lists.changed() {
        for &y in members {
            round.emit(y, ty, class);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::rules::testing;

    #[test]
    fn cls_oo_types_each_member() {
        let run = testing::run(&["class"], "ex:C owl:oneOf ( ex:a ex:b ) .");
        assert!(run.has("ex:a rdf:type ex:C"));
        assert!(run.has("ex:b rdf:type ex:C"));
    }

    /// The list is read as the W3C `LIST[...]` pattern: a cyclic list, or
    /// a list whose path to `rdf:nil` passes a cell without an `rdf:first`,
    /// types nothing.
    #[test]
    fn cls_oo_reads_only_lists_that_reach_nil() {
        let run = testing::run(
            &["class"],
            "ex:C owl:oneOf ex:l1 . ex:l1 rdf:first ex:a ; rdf:rest ex:l2 . ex:l2 rdf:first ex:b ; rdf:rest ex:l1 .
             ex:D owl:oneOf ex:m1 . ex:m1 rdf:first ex:c ; rdf:rest ex:m2 . ex:m2 rdf:rest rdf:nil .",
        );
        for t in ["ex:a rdf:type ex:C", "ex:b rdf:type ex:C", "ex:c rdf:type ex:D"] {
            assert!(!run.has(t), "{t}");
        }
    }

    /// A list completed by a later run (an incremental one) types its
    /// members then; a member made equal to another term types it too.
    #[test]
    fn cls_oo_reads_a_list_completed_later() {
        let mut run = testing::run(
            &["class", "equality"],
            "ex:C owl:oneOf ex:l1 . ex:l1 rdf:first ex:a ; rdf:rest ex:l2 . ex:l2 rdf:first ex:b .",
        );
        assert!(!run.has("ex:a rdf:type ex:C"));
        run.add("ex:l2 rdf:rest rdf:nil . ex:b owl:sameAs ex:e .");
        for t in ["ex:a rdf:type ex:C", "ex:b rdf:type ex:C", "ex:e rdf:type ex:C"] {
            assert!(run.has(t), "{t}");
        }
    }
}
