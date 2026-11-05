//! The class family (ticket 07): cax-sco, cax-eqc1/2, cls-thing,
//! cls-nothing1, cls-int1/2, cls-uni, cls-svf1/2, cls-avf, cls-hv1/2,
//! scm-sco and scm-eqc1, in their W3C forms (`docs/specs/v1.md`, the rule
//! set). The contract is the reference evaluator
//! (`crates/reference/src/evaluate.rs`, `classes()`), which equals Nemo
//! running `oracles/nemo/owl2rl.rls`.
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
//! Constants are bare vocabulary ids for now; ticket 11 reads them through
//! the round's canon map (ADR 0003).

mod lists;
mod restrictions;
#[cfg(test)]
mod tests;

use super::Context;
use crate::engine::{Pair, Round, Variable, gallop, join};
use crate::term::{TermId, vocab};

/// The heads of cls-thing and cls-nothing1.
pub(crate) const FACTS: &[[TermId; 3]] = &[
    [vocab::OWL_THING, vocab::RDF_TYPE, vocab::OWL_CLASS],
    [vocab::OWL_NOTHING, vocab::RDF_TYPE, vocab::OWL_CLASS],
];

const TYPE: u32 = vocab::RDF_TYPE.0;
const SUB_CLASS_OF: u32 = vocab::RDFS_SUB_CLASS_OF.0;
const EQUIVALENT_CLASS: u32 = vocab::OWL_EQUIVALENT_CLASS.0;

/// One round of the class rules.
pub(crate) fn rules(round: &mut Round<'_>, _cx: &mut Context<'_>) {
    // `rdf:type` by object, (class, individual): every rule that reads the
    // instances of a class reads them here.
    let instances = round.by_object(TYPE);
    cax_sco(round, instances);
    cax_eqc1(round, instances);
    cax_eqc2(round, instances);
    scm_sco(round);
    scm_eqc1(round);
    lists::rules(round, instances);
    restrictions::rules(round, instances);
}

/// cax-sco: `c1 rdfs:subClassOf c2`, `x rdf:type c1` → `x rdf:type c2`.
/// Join on c1: `subClassOf` by subject with `rdf:type` by object.
fn cax_sco(round: &mut Round<'_>, instances: Option<&Variable<Pair>>) {
    let (Some(sco), Some(instances)) = (round.by_subject(SUB_CLASS_OF), instances) else {
        return;
    };
    join(sco, instances, |_, c2, x| round.emit(x, TYPE, c2));
}

/// cax-eqc1: `c1 owl:equivalentClass c2`, `x rdf:type c1` → `x rdf:type c2`.
/// Join on c1: `equivalentClass` by subject with `rdf:type` by object.
fn cax_eqc1(round: &mut Round<'_>, instances: Option<&Variable<Pair>>) {
    let (Some(eqc), Some(instances)) = (round.by_subject(EQUIVALENT_CLASS), instances) else {
        return;
    };
    join(eqc, instances, |_, c2, x| round.emit(x, TYPE, c2));
}

/// cax-eqc2: `c1 owl:equivalentClass c2`, `x rdf:type c2` → `x rdf:type c1`.
/// Join on c2: `equivalentClass` by object with `rdf:type` by object.
fn cax_eqc2(round: &mut Round<'_>, instances: Option<&Variable<Pair>>) {
    let Some(instances) = instances else { return };
    let Some(eqc) = round.by_object(EQUIVALENT_CLASS) else {
        return;
    };
    join(eqc, instances, |_, c1, x| round.emit(x, TYPE, c1));
}

/// scm-sco: `c1 rdfs:subClassOf c2`, `c2 rdfs:subClassOf c3` →
/// `c1 rdfs:subClassOf c3`. Join on c2: `subClassOf` by object with
/// `subClassOf` by subject.
fn scm_sco(round: &mut Round<'_>) {
    let Some(sco) = round.by_subject(SUB_CLASS_OF) else {
        return;
    };
    let Some(sco_by_object) = round.by_object(SUB_CLASS_OF) else {
        return;
    };
    join(sco_by_object, sco, |_, c1, c3| round.emit(c1, SUB_CLASS_OF, c3));
}

/// scm-eqc1: `c1 owl:equivalentClass c2` → `c1 rdfs:subClassOf c2` and
/// `c2 rdfs:subClassOf c1`. Reads the recent `equivalentClass` pairs.
fn scm_eqc1(round: &mut Round<'_>) {
    let Some(eqc) = round.by_subject(EQUIVALENT_CLASS) else {
        return;
    };
    for &(c1, c2) in eqc.recent().iter() {
        round.emit(c1, SUB_CLASS_OF, c2);
        round.emit(c2, SUB_CLASS_OF, c1);
    }
}

/// The run of pairs whose key is `k` in a sorted part.
fn range(part: &[Pair], k: u32) -> &[Pair] {
    let start = part.partition_point(|t| t.0 < k);
    let rest = &part[start..];
    let after = gallop(rest, |t| t.0 <= k);
    &rest[..rest.len() - after.len()]
}
