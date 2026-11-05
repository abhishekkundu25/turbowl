//! The equality family (ticket 11): eq-sym, eq-trans, eq-rep-s, eq-rep-p
//! and eq-rep-o, as merges in the store's canon (ADR 0003), and the
//! expansion of representatives on output.
//!
//! # The rules
//!
//! Each recent `x owl:sameAs y` with `x` and `y` different asks the store
//! to merge their classes ([`Round::merge`]). The store keeps every tuple
//! in representatives, so after the merge the class is one id: eq-rep-s,
//! eq-rep-p and eq-rep-o hold because a triple about any member is a
//! triple about the representative, and eq-sym and eq-trans because the
//! `sameAs` triples of the class are one stored triple, `r owl:sameAs r`.
//! No triple is derived. eq-ref is not in the rule set, so a term is equal
//! to itself only where a triple says so: `x owl:sameAs x` is stored as it
//! came (a class of one), and a class of two or more holds `r owl:sameAs r`
//! because the triple that merged it is rewritten to it.
//!
//! The constants the rules name (`owl:sameAs` here, `rdf:type` and the rest
//! in the other families) are read through the round
//! ([`Round::constant`]), so `ex:same owl:sameAs owl:sameAs` makes `ex:same`
//! triples merge too.
//!
//! # Output
//!
//! The closure the W3C eq-* rules give holds, for each stored triple
//! `s p o`, every `s' p' o'` with `s'`, `p'` and `o'` members of the
//! classes of `s`, `p` and `o` ([`expanded`]); with `r owl:sameAs r`, that
//! is every `owl:sameAs` triple among the members of a class. The stored
//! triples are in representatives and the classes are disjoint, so the
//! expansions are disjoint too, and the RDF triples among them can be
//! counted without listing them ([`Weights`]).
//!
//! A finding is expanded the same way: each of its terms to every member
//! of its class ([`expand_finding`]), since the rule that concludes it
//! matches each choice of members in the expanded closure. dt-not-type is
//! the exception: its rule reads the literal itself (its datatype and
//! lexical form) and names a datatype constant, so the datatype family
//! reports the literal members it checked, and they stay as reported.

use super::{Context, Finding};
use crate::engine::{Canon, Round};
use crate::term::{Dictionary, Kind, TermId, vocab};
use rustc_hash::FxHashMap;

/// The equality rules have no empty-body rule (eq-ref is not in the set).
pub(crate) const FACTS: &[[TermId; 3]] = &[];

/// A canon in which no two terms are equal yet, whose constants are the
/// vocabulary (the terms the rules name, interned first), and which
/// watches `rdf:nil`, the one constant the rules test for identity (where a
/// list ends: `rules/class/lists.rs`).
pub(crate) fn canon() -> Canon {
    Canon::new(vocab::IRIS.len() as u32, &[vocab::RDF_NIL.0])
}

/// One round of the equality rules: a merge for each recent `owl:sameAs`
/// between two different representatives.
pub(crate) fn rules(round: &mut Round<'_>, _cx: &mut Context<'_>) {
    let same_as = round.constant(vocab::OWL_SAME_AS.0);
    let Some(table) = round.by_subject(same_as) else {
        return;
    };
    for &(x, y) in table.recent().iter() {
        if x != y {
            round.merge(x, y);
        }
    }
}

/// Every triple the stored triples stand for: each `(s, p, o)` once for
/// every member of the class of `s`, of `p` and of `o`.
pub(crate) fn expanded<'a>(
    canon: &'a Canon,
    triples: impl Iterator<Item = (u32, u32, u32)> + 'a,
) -> impl Iterator<Item = (u32, u32, u32)> + 'a {
    triples.flat_map(move |(s, p, o)| {
        let (ps, os) = (canon.members(p), canon.members(o));
        canon
            .members(s)
            .iter()
            .flat_map(move |s| ps.iter().flat_map(move |p| os.iter().map(move |o| (s, p, o))))
    })
}

/// A finding whose terms are representatives now (dt-not-type's are kept:
/// see the module doc).
pub(crate) fn canonical(canon: &Canon, finding: &mut Finding) {
    if finding.code != "dt-not-type" {
        for t in &mut finding.terms {
            *t = canon.find(*t);
        }
    }
}

/// Pushes every finding `finding` (in representatives) stands for: one for
/// each choice of a member of each term's class.
pub(crate) fn expand_finding(canon: &Canon, finding: &Finding, out: &mut Vec<Finding>) {
    if finding.code == "dt-not-type" || finding.terms.iter().all(|&t| canon.members(t).len() == 1) {
        out.push(finding.clone());
        return;
    }
    let start = out.len();
    out.push(Finding {
        code: finding.code,
        terms: Vec::with_capacity(finding.terms.len()),
    });
    for &t in &finding.terms {
        let partial = out.split_off(start);
        for f in partial {
            for m in canon.members(t).iter() {
                let mut terms = f.terms.clone();
                terms.push(m);
                out.push(Finding { code: f.code, terms });
            }
        }
    }
}

/// How many RDF triples (a subject that is not a literal, an IRI
/// predicate) a stored triple stands for once expanded: the product of the
/// sizes of its classes, each counting only the members that can stand in
/// its position.
#[derive(Debug, Default)]
pub(crate) struct Weights {
    /// For the representative of each class of two or more: its members,
    /// those that are not literals, and its IRIs.
    classes: FxHashMap<u32, (usize, usize, usize)>,
}

impl Weights {
    /// The weights under `canon`. Costs one look at each member of a class.
    pub(crate) fn new(canon: &Canon, terms: &Dictionary) -> Self {
        let classes = canon
            .classes()
            .map(|(rep, members)| {
                let kinds = members.iter().map(|&m| terms.kind(TermId(m)));
                let (mut not_literal, mut iri) = (0, 0);
                for kind in kinds {
                    not_literal += usize::from(kind != Kind::Literal);
                    iri += usize::from(kind == Kind::Iri);
                }
                (rep, (members.len(), not_literal, iri))
            })
            .collect();
        Self { classes }
    }

    /// True when every class has one member: a triple stands for itself.
    pub(crate) fn is_trivial(&self) -> bool {
        self.classes.is_empty()
    }

    /// The RDF triples `(s, p, o)` stands for.
    #[inline]
    pub(crate) fn of(&self, terms: &Dictionary, s: u32, p: u32, o: u32) -> usize {
        let subjects = match self.classes.get(&s) {
            Some(c) => c.1,
            None => usize::from(terms.kind(TermId(s)) != Kind::Literal),
        };
        if subjects == 0 {
            return 0;
        }
        let predicates = match self.classes.get(&p) {
            Some(c) => c.2,
            None => usize::from(terms.kind(TermId(p)) == Kind::Iri),
        };
        let objects = self.classes.get(&o).map_or(1, |c| c.0);
        subjects * predicates * objects
    }
}

#[cfg(test)]
mod tests;
