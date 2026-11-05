//! The property family (ticket 08), in the core: prp-dom, prp-rng, prp-fp,
//! prp-ifp, prp-symp, prp-trp, prp-spo1, prp-eqp1/2 and prp-inv1/2, in their
//! W3C forms, over generalized triples. prp-spo2 is a family of its own,
//! `property-chains`, under the switch of that name: the [`chain`] module.
//!
//! # Semi-naive on both sides
//!
//! Every rule here has a schema atom that names the property `p` (`p
//! rdfs:domain c`, `p rdf:type owl:FunctionalProperty`, ...) and one or two
//! atoms over `p`'s own table. The schema atom is read from the subject
//! index of its predicate (`rdfs:domain`, ...) or, for the property
//! characteristics, from the object index of `rdf:type`
//! (`(owl:FunctionalProperty, p)`). A schema tuple new this round joins
//! with all of `p`'s table; one seen before joins only with `p`'s recent
//! part (for a self-join, the engine's [`join`]: recent × stable, stable ×
//! recent, recent × recent). So each derivation is made once over a run.
//! A property's table is looked up once per schema tuple, never inside a
//! loop over its pairs.
//!
//! Each round walks the family's schema tuples (a few dozen on Brick) to
//! find the properties whose tables have a recent part; it never scans an
//! instance table it does not join.
//!
//! # Join plans
//!
//! - prp-dom, prp-rng, prp-symp, prp-spo1, prp-eqp1/2, prp-inv1/2: copies,
//!   no join. Each pair of `p` (its recent part, or all of it when the
//!   schema tuple is recent) is emitted rewritten.
//! - prp-fp: `p`'s subject index joined with itself on the subject.
//! - prp-ifp: `p`'s object index joined with itself on the object, so only
//!   subjects with an equal object are made the same.
//! - prp-trp: `p`'s object index `(y, x)` joined with its subject index
//!   `(y, z)` on `y`, while that join derives at most as many triples as
//!   `p`'s table holds (counted first, without making them). Past that,
//!   `p`'s whole table is closed in one step
//!   ([`transitive_closure`]), which reports each missing pair once. The
//!   join derives a pair once per middle term: on a strongly connected
//!   component of k nodes (a symmetric, transitive property such as
//!   OWL2Bench's `hasSameHomeTownWith`), k³ derivations for k² pairs, all
//!   held until the round ends (ticket 25). Either way, every pair that
//!   joins a recent one is derived this round, so the run stays complete.
//!
//! An object index ([`Round::by_object`]) is built the round after a rule
//! first asks for it, with every pair recent then; a rule skips its join
//! until it exists and loses nothing.
//!
//! prp-fp and prp-ifp derive `y owl:sameAs y` too (the W3C rules have no
//! inequality); the equality family merges what they make equal.
//!
//! Every constant is read through the round's canon ([`Round::constant`],
//! ADR 0003), so it keeps its meaning when `owl:sameAs` makes it equal to
//! another term.

pub(crate) mod chain;
#[cfg(test)]
mod tests;

use super::Context;
use crate::engine::{Pair, Round, Variable, join, join_count, join_views, join_views_count, transitive_closure};
use crate::term::{TermId, vocab};

/// The property rules have no empty-body rule.
pub(crate) const FACTS: &[[TermId; 3]] = &[];

/// One round of the property rules.
pub(crate) fn rules(round: &mut Round<'_>, _cx: &mut Context<'_>) {
    copies(round);
    characteristics(round);
}

/// A variable's sorted parts, each with whether it is the recent one.
fn tagged(v: &Variable<Pair>) -> impl Iterator<Item = (&[Pair], bool)> {
    v.stable()
        .iter()
        .map(|b| (&b[..], false))
        .chain(Some((&v.recent()[..], true)))
}

/// The pairs of `table` a rule joins this round: all of them when the
/// schema tuple that names the table is `recent`, else its recent part.
fn pairs_for(table: &Variable<Pair>, recent: bool) -> impl Iterator<Item = &Pair> {
    let stable: &[_] = if recent { table.stable() } else { &[] };
    stable.iter().flat_map(|b| b.iter()).chain(table.recent().iter())
}

/// A schema tuple that names a property: the property's subject index
/// (`table`), the property itself (`key`), the tuple's other term
/// (`other`), and whether the tuple is new this round (`recent`).
struct Axiom<'a> {
    table: &'a Variable<Pair>,
    key: u32,
    other: u32,
    recent: bool,
}

/// Calls `f` for each tuple `(key, other)` of the schema predicate `schema`
/// (read the other way round, `(other, key)`, when `reversed`) whose
/// property `key` has a table. A tuple seen before is skipped when `key`'s
/// table has no recent part: it has nothing new to join.
fn each_axiom<'a>(round: &mut Round<'a>, schema: u32, reversed: bool, mut f: impl FnMut(&mut Round<'a>, Axiom<'a>)) {
    let store = round.store();
    let Some(axioms) = round.by_subject(schema) else {
        return;
    };
    for (part, recent) in tagged(axioms) {
        // Consecutive tuples often share their key: look its table up once.
        let mut last: Option<(u32, Option<&'a Variable<Pair>>)> = None;
        for &(s, o) in part {
            let (key, other) = if reversed { (o, s) } else { (s, o) };
            let table = match last {
                Some((k, table)) if k == key => table,
                _ => {
                    let table = store.table(key).map(|t| t.by_subject());
                    last = Some((key, table));
                    table
                }
            };
            if let Some(table) = table
                && (recent || !table.recent().is_empty())
            {
                f(
                    round,
                    Axiom {
                        table,
                        key,
                        other,
                        recent,
                    },
                );
            }
        }
    }
}

/// prp-dom, prp-rng, prp-spo1, prp-eqp1/2, prp-inv1/2: rules that copy a
/// property's pairs, rewritten, with no join.
fn copies(round: &mut Round<'_>) {
    let ty = round.constant(vocab::RDF_TYPE.0);
    let equivalent = round.constant(vocab::OWL_EQUIVALENT_PROPERTY.0);
    let inverse = round.constant(vocab::OWL_INVERSE_OF.0);
    // prp-dom: p rdfs:domain c, x p y => x rdf:type c. Pairs are sorted by
    // subject within a part, so a subject's run gives one triple.
    each_axiom(round, round.constant(vocab::RDFS_DOMAIN.0), false, |round, a| {
        let mut last = None;
        for &(x, _) in pairs_for(a.table, a.recent) {
            if last != Some(x) {
                round.emit(x, ty, a.other);
                last = Some(x);
            }
        }
    });
    // prp-rng: p rdfs:range c, x p y => y rdf:type c.
    each_axiom(round, round.constant(vocab::RDFS_RANGE.0), false, |round, a| {
        for &(_, y) in pairs_for(a.table, a.recent) {
            round.emit(y, ty, a.other);
        }
    });
    // prp-spo1: p1 rdfs:subPropertyOf p2, x p1 y => x p2 y.
    each_axiom(
        round,
        round.constant(vocab::RDFS_SUB_PROPERTY_OF.0),
        false,
        |round, a| copy(round, &a, false),
    );
    // prp-eqp1: p1 owl:equivalentProperty p2, x p1 y => x p2 y.
    each_axiom(round, equivalent, false, |round, a| copy(round, &a, false));
    // prp-eqp2: p1 owl:equivalentProperty p2, x p2 y => x p1 y.
    each_axiom(round, equivalent, true, |round, a| copy(round, &a, false));
    // prp-inv1: p1 owl:inverseOf p2, x p1 y => y p2 x.
    each_axiom(round, inverse, false, |round, a| copy(round, &a, true));
    // prp-inv2: p1 owl:inverseOf p2, x p2 y => y p1 x.
    each_axiom(round, inverse, true, |round, a| copy(round, &a, true));
}

/// Emits each pair of the axiom's table that the rule joins this round as a
/// triple of `a.other`, the pair `reversed` or not. A copy of a table into
/// itself (`p rdfs:subPropertyOf p`) derives nothing new and is skipped.
fn copy(round: &mut Round<'_>, a: &Axiom<'_>, reversed: bool) {
    let to = a.other;
    if reversed {
        for &(x, y) in pairs_for(a.table, a.recent) {
            round.emit(y, to, x);
        }
    } else if a.key != to {
        for &(x, y) in pairs_for(a.table, a.recent) {
            round.emit(x, to, y);
        }
    }
}

/// Calls `f(round, p, recent)` for each `p rdf:type class` (`class` a
/// constant, read through the round), `recent` when that triple is new this
/// round. Reads `rdf:type`'s object index; the round it is first asked for
/// there is none, and nothing is called (the next round sees every typing
/// as recent).
fn each_typed<'a>(round: &mut Round<'a>, class: u32, mut f: impl FnMut(&mut Round<'a>, u32, bool)) {
    let class = round.constant(class);
    let Some(types) = round.by_object(round.constant(vocab::RDF_TYPE.0)) else {
        return;
    };
    for (part, recent) in tagged(types) {
        let start = part.partition_point(|t| t.0 < class);
        for &(_, p) in part[start..].iter().take_while(|t| t.0 == class) {
            f(round, p, recent);
        }
    }
}

/// prp-fp, prp-ifp, prp-symp, prp-trp: the rules on a property typed with
/// one of the OWL property characteristics.
fn characteristics(round: &mut Round<'_>) {
    let same_as = round.constant(vocab::OWL_SAME_AS.0);
    // prp-fp: x p y1, x p y2 => y1 owl:sameAs y2.
    each_typed(round, vocab::OWL_FUNCTIONAL_PROPERTY.0, |round, p, recent| {
        let Some(so) = round.by_subject(p) else { return };
        if recent {
            join_views(&so.all(), &so.all(), |_, y1, y2| round.emit(y1, same_as, y2));
        } else {
            join(so, so, |_, y1, y2| round.emit(y1, same_as, y2));
        }
    });
    // prp-ifp: x1 p y, x2 p y => x1 owl:sameAs x2.
    each_typed(round, vocab::OWL_INVERSE_FUNCTIONAL_PROPERTY.0, |round, p, recent| {
        let Some(os) = round.by_object(p) else { return };
        if recent {
            join_views(&os.all(), &os.all(), |_, x1, x2| round.emit(x1, same_as, x2));
        } else {
            join(os, os, |_, x1, x2| round.emit(x1, same_as, x2));
        }
    });
    // prp-symp: x p y => y p x.
    each_typed(round, vocab::OWL_SYMMETRIC_PROPERTY.0, |round, p, recent| {
        let Some(so) = round.by_subject(p) else { return };
        for &(x, y) in pairs_for(so, recent) {
            round.emit(y, p, x);
        }
    });
    // prp-trp: x p y, y p z => x p z, through p's object index; or p's
    // table closed in one step, when the join would derive more triples
    // than the table holds (see the module doc).
    each_typed(round, vocab::OWL_TRANSITIVE_PROPERTY.0, |round, p, recent| {
        let (Some(so), Some(os)) = (round.by_subject(p), round.by_object(p)) else {
            return;
        };
        let derivations = if recent {
            join_views_count(&os.all(), &so.all())
        } else {
            join_count(os, so)
        };
        if derivations > so.len() {
            transitive_closure(&so.all(), |x, z| round.emit(x, p, z));
        } else if recent {
            join_views(&os.all(), &so.all(), |_, x, z| round.emit(x, p, z));
        } else {
            join(os, so, |_, x, z| round.emit(x, p, z));
        }
    });
}
