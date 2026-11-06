//! The class family (ticket 07), in the core: cax-sco, cax-eqc1/2,
//! cls-thing, cls-nothing1, cls-int1/2, cls-uni, cls-svf1/2, cls-avf,
//! cls-hv1/2, cls-oo, scm-sco and scm-eqc1, in their W3C forms
//! (`docs/specs/v1.md`, the rule set). The contract is the reference evaluator
//! (`crates/reference/src/rules/core.rs`), which equals Nemo
//! running the rule program of `oracles/rules/`.
//!
//! Every rule is semi-naive: each round it joins the recent part of one body
//! relation with the rest, so each rule instance is considered once over a
//! run (`engine/mod.rs`). Schema-sized structures (the lists of
//! `owl:intersectionOf` and `owl:unionOf`, the restrictions with their
//! `owl:onProperty`) are rebuilt from the store each round, since a family
//! keeps no state; per-instance work joins only what is new.
//!
//! - This file: the class axioms and schema rules, each its own function,
//!   because the experiments of `docs/research/sota-algorithms-2026-11.md`
//!   §10 replace them one at a time (E2 scm-sco, E8 cax-eqc1/2, E1 cax-sco).
//! - `class/lists.rs`: cls-int1, cls-int2, cls-uni.
//! - `class/restrictions.rs`: cls-svf1, cls-svf2, cls-avf, cls-hv1, cls-hv2.
//! - `class/enumerations.rs`: cls-oo.
//! - cls-thing and cls-nothing1 have empty bodies: they are [`FACTS`].
//!
//! Generalized triples: no rule filters what it derives. cls-avf types a
//! literal, cls-hv1 derives `u p y` for whatever `p` the schema binds.
//!
//! Object indexes are lazy ([`Round::by_object`]): the first request returns
//! `None` and the next round holds the whole index as recent. A rule that
//! gets `None` skips the joins that need the index that round. Those joins
//! read the index's recent part, or its stable part beside a join that reads
//! its recent part, so the next round, where all of it is recent, makes up
//! for what was skipped.
//!
//! Constants (`rdf:type`, `owl:Thing`, `rdf:nil`, ...) are read through the
//! round's canon ([`Round::constant`], ADR 0003), so they keep their meaning
//! when `owl:sameAs` makes one equal to another term.

mod enumerations;
mod lists;
mod restrictions;
#[cfg(test)]
mod tests;

use super::Context;
use crate::engine::{Pair, Round, Variable, gallop};
use crate::term::{TermId, vocab};

/// The heads of cls-thing and cls-nothing1.
pub(crate) const FACTS: &[[TermId; 3]] = &[
    [vocab::OWL_THING, vocab::RDF_TYPE, vocab::OWL_CLASS],
    [vocab::OWL_NOTHING, vocab::RDF_TYPE, vocab::OWL_CLASS],
];

/// The constants this file's rules name, through the round's canon.
#[derive(Debug, Clone, Copy)]
struct Vocab {
    ty: u32,
    sub_class_of: u32,
    equivalent_class: u32,
}

/// One round of the class rules.
pub(crate) fn rules(round: &mut Round<'_>, _cx: &mut Context<'_>) {
    let v = Vocab {
        ty: round.constant(vocab::RDF_TYPE.0),
        sub_class_of: round.constant(vocab::RDFS_SUB_CLASS_OF.0),
        equivalent_class: round.constant(vocab::OWL_EQUIVALENT_CLASS.0),
    };
    // `rdf:type` by object, (class, individual): every rule that reads the
    // instances of a class reads them here.
    let instances = round.by_object(v.ty);
    cax_sco(round, v, instances);
    cax_eqc1(round, v, instances);
    cax_eqc2(round, v, instances);
    scm_sco(round, v);
    scm_eqc1(round, v);
    lists::rules(round, v.ty, instances);
    restrictions::rules(round, v.ty, instances);
    enumerations::rules(round, v.ty);
}

/// cax-sco: `c1 rdfs:subClassOf c2`, `x rdf:type c1` → `x rdf:type c2`.
/// Join on c1: `subClassOf` by subject with `rdf:type` by object.
fn cax_sco(round: &mut Round<'_>, v: Vocab, instances: Option<&Variable<Pair>>) {
    let (Some(sco), Some(instances)) = (round.by_subject(v.sub_class_of), instances) else {
        return;
    };
    round.join(sco, instances, |round, _, c2, x| round.emit(x, v.ty, c2));
}

/// cax-eqc1: `c1 owl:equivalentClass c2`, `x rdf:type c1` → `x rdf:type c2`.
/// Join on c1: `equivalentClass` by subject with `rdf:type` by object.
fn cax_eqc1(round: &mut Round<'_>, v: Vocab, instances: Option<&Variable<Pair>>) {
    let (Some(eqc), Some(instances)) = (round.by_subject(v.equivalent_class), instances) else {
        return;
    };
    round.join(eqc, instances, |round, _, c2, x| round.emit(x, v.ty, c2));
}

/// cax-eqc2: `c1 owl:equivalentClass c2`, `x rdf:type c2` → `x rdf:type c1`.
/// Join on c2: `equivalentClass` by object with `rdf:type` by object.
fn cax_eqc2(round: &mut Round<'_>, v: Vocab, instances: Option<&Variable<Pair>>) {
    let Some(instances) = instances else { return };
    let Some(eqc) = round.by_object(v.equivalent_class) else {
        return;
    };
    round.join(eqc, instances, |round, _, c1, x| round.emit(x, v.ty, c1));
}

/// scm-sco: `c1 rdfs:subClassOf c2`, `c2 rdfs:subClassOf c3` →
/// `c1 rdfs:subClassOf c3`. Join on c2: `subClassOf` by object with
/// `subClassOf` by subject.
fn scm_sco(round: &mut Round<'_>, v: Vocab) {
    let Some(sco) = round.by_subject(v.sub_class_of) else {
        return;
    };
    let Some(sco_by_object) = round.by_object(v.sub_class_of) else {
        return;
    };
    round.join(sco_by_object, sco, |round, _, c1, c3| {
        round.emit(c1, v.sub_class_of, c3)
    });
}

/// scm-eqc1: `c1 owl:equivalentClass c2` → `c1 rdfs:subClassOf c2` and
/// `c2 rdfs:subClassOf c1`. Reads the recent `equivalentClass` pairs.
fn scm_eqc1(round: &mut Round<'_>, v: Vocab) {
    let Some(eqc) = round.by_subject(v.equivalent_class) else {
        return;
    };
    for &(c1, c2) in eqc.recent().iter() {
        round.emit(c1, v.sub_class_of, c2);
        round.emit(c2, v.sub_class_of, c1);
    }
}

/// The run of pairs whose key is `k` in a sorted part.
fn range(part: &[Pair], k: u32) -> &[Pair] {
    let start = part.partition_point(|t| t.0 < k);
    let rest = &part[start..];
    let after = gallop(rest, |t| t.0 <= k);
    &rest[..rest.len() - after.len()]
}
