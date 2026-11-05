//! The trivial family (switch `trivial`, off by default): the rules whose
//! triples hold of every term, class, property or literal (OWL 2 Profiles,
//! Tables 4, 5, 8 and 9).
//!
//! - eq-ref: `x owl:sameAs x` for every term. An output rule (the phase 1
//!   spec, Implementation Decisions): [`same_as_itself`] lists the terms
//!   the reasoner adds it for when the closure is read, and the summary
//!   counts them. The store keeps it only where a rule could read it (see
//!   below).
//! - scm-cls: for each `c rdf:type owl:Class`, `c rdfs:subClassOf c`,
//!   `c owl:equivalentClass c`, `c rdfs:subClassOf owl:Thing` and
//!   `owl:Nothing rdfs:subClassOf c`.
//! - scm-op and scm-dp: for each `p rdf:type owl:ObjectProperty` and
//!   `p rdf:type owl:DatatypeProperty`, `p rdfs:subPropertyOf p` and
//!   `p owl:equivalentProperty p`.
//! - prp-ap: each annotation property of OWL 2 (`rdfs:label`,
//!   `rdfs:comment`, `rdfs:seeAlso`, `rdfs:isDefinedBy`, `owl:deprecated`,
//!   `owl:versionInfo`, `owl:priorVersion`, `owl:backwardCompatibleWith`,
//!   `owl:incompatibleWith`) typed `owl:AnnotationProperty`: [`FACTS`].
//! - dt-type1: each datatype of the datatype map (`datatype/map.rs`) typed
//!   `rdfs:Datatype`: [`FACTS`].
//! - dt-type2: each literal typed with every datatype whose value space
//!   holds its value. Version 2's ticket 10 adds it.
//!
//! scm-cls, scm-op and scm-dp read the recent part of `rdf:type` by object
//! (built lazily: the round after the first request holds all of it as
//! recent), so each typing is read once over a run.
//!
//! # eq-ref and the other rules
//!
//! With representatives (ADR 0003), the store holds `r owl:sameAs r` for
//! every class of two or more terms (the triple that merged it, rewritten),
//! and its expansion gives every member's reflexive pair. So eq-ref adds a
//! triple only for a representative whose `r owl:sameAs r` is not stored,
//! and [`same_as_itself`] lists those.
//!
//! The other rules never see these triples. That loses nothing as long as
//! no rule reads `owl:sameAs` as a property the schema names: eq-rep-s,
//! eq-rep-p and eq-rep-o copy a triple onto itself, eq-sym and eq-trans add
//! nothing to `x owl:sameAs x`, and eq-diff1, eq-diff2 and eq-diff3 treat
//! every term as `owl:sameAs` itself when `trivial` is on (`checks.rs`). A
//! rule that binds a property from the schema (prp-dom on `owl:sameAs
//! rdfs:domain c`, cls-svf1 on `r owl:onProperty owl:sameAs`, a chain or a
//! key listing `owl:sameAs`, ...) does read them, and each such rule finds
//! `owl:sameAs` as the subject or the object of a triple. So once the
//! representative of `owl:sameAs` is the subject or the object of a stored
//! triple, this family stores eq-ref ([`eq_ref`]): `t owl:sameAs t` for
//! every term `t` of the closure, literals included. Real data almost
//! never does this; the generator does.

use super::Context;
use super::datatype::map::Datatype;
use crate::engine::{Pair, Round, Store, gallop};
use crate::term::{Dictionary, Kind, TermId, vocab};

/// The annotation properties of prp-ap (OWL 2 Profiles, Table 5).
const ANNOTATION_PROPERTIES: [TermId; 9] = [
    vocab::RDFS_LABEL,
    vocab::RDFS_COMMENT,
    vocab::RDFS_SEE_ALSO,
    vocab::RDFS_IS_DEFINED_BY,
    vocab::OWL_DEPRECATED,
    vocab::OWL_VERSION_INFO,
    vocab::OWL_PRIOR_VERSION,
    vocab::OWL_BACKWARD_COMPATIBLE_WITH,
    vocab::OWL_INCOMPATIBLE_WITH,
];

/// prp-ap's facts, then dt-type1's, one for each datatype of the map.
const FACT_TABLE: [[TermId; 3]; ANNOTATION_PROPERTIES.len() + Datatype::ALL.len()] = {
    let mut facts = [[vocab::RDF_TYPE; 3]; ANNOTATION_PROPERTIES.len() + Datatype::ALL.len()];
    let mut i = 0;
    while i < ANNOTATION_PROPERTIES.len() {
        facts[i] = [
            ANNOTATION_PROPERTIES[i],
            vocab::RDF_TYPE,
            vocab::OWL_ANNOTATION_PROPERTY,
        ];
        i += 1;
    }
    let mut d = 0;
    while d < Datatype::ALL.len() {
        facts[i + d] = [Datatype::ALL[d].id(), vocab::RDF_TYPE, vocab::RDFS_DATATYPE];
        d += 1;
    }
    facts
};

/// The heads of the group's empty-body rules, prp-ap and dt-type1. The
/// reasoner inserts them only when the `trivial` switch is on.
pub(crate) const FACTS: &[[TermId; 3]] = &FACT_TABLE;

/// One round of the trivial rules.
pub(crate) fn rules(round: &mut Round<'_>, _cx: &mut Context<'_>) {
    let ty = round.constant(vocab::RDF_TYPE.0);
    if let Some(types) = round.by_object(ty) {
        let recent = types.recent();
        let c = |id: TermId| round.constant(id.0);
        let (sub_class_of, equivalent_class) = (c(vocab::RDFS_SUB_CLASS_OF), c(vocab::OWL_EQUIVALENT_CLASS));
        let (thing, nothing) = (c(vocab::OWL_THING), c(vocab::OWL_NOTHING));
        let (sub_property_of, equivalent_property) =
            (c(vocab::RDFS_SUB_PROPERTY_OF), c(vocab::OWL_EQUIVALENT_PROPERTY));
        let (class, object_property, datatype_property) = (
            c(vocab::OWL_CLASS),
            c(vocab::OWL_OBJECT_PROPERTY),
            c(vocab::OWL_DATATYPE_PROPERTY),
        );
        // scm-cls.
        for &(_, c) in with_key(recent, class) {
            round.emit(c, sub_class_of, c);
            round.emit(c, equivalent_class, c);
            round.emit(c, sub_class_of, thing);
            round.emit(nothing, sub_class_of, c);
        }
        // scm-op, scm-dp.
        for kind in [object_property, datatype_property] {
            for &(_, p) in with_key(recent, kind) {
                round.emit(p, sub_property_of, p);
                round.emit(p, equivalent_property, p);
            }
        }
    }
    eq_ref(round);
}

/// The pairs of a sorted slice whose first field is `key`.
fn with_key(pairs: &[Pair], key: u32) -> &[Pair] {
    let from = gallop(pairs, |t| t.0 < key);
    &from[..from.partition_point(|t| t.0 == key)]
}

/// eq-ref in the store, once a rule can read it (see the module doc).
///
/// - Once `same owl:sameAs same` is in a stable part of the store (`same`
///   is the representative of `owl:sameAs`), eq-ref is stored:
///   `t owl:sameAs t` for every term of the recent triples.
/// - Before that, when `same` is the subject or the object of a recent
///   triple: `t owl:sameAs t` for every term of the store, `same` included.
///
/// `same owl:sameAs same` names `same` as a subject, so in the round in
/// which it is recent, the second case holds: whenever it is stable, every
/// term the store held then has its triple. Each round reads the recent
/// triples once (one pass over what a run derives); the whole store is
/// read in at most a round or two of a run.
fn eq_ref(round: &mut Round<'_>) {
    let same = round.constant(vocab::OWL_SAME_AS.0);
    let store = round.store();
    let stored = store
        .table(same)
        .is_some_and(|t| t.by_subject().stable().iter().any(|part| part.contains(&(same, same))));
    let terms = if stored {
        terms(store, true)
    } else if store.tables().any(|(_, table)| {
        let recent = table.by_subject().recent();
        !with_key(recent, same).is_empty() || recent.iter().any(|&(_, o)| o == same)
    }) {
        let mut all = terms(store, false);
        all.insert(same);
        all
    } else {
        return;
    };
    for t in terms.iter() {
        round.emit(t, same, t);
    }
}

/// The distinct terms of the store's triples, in any position: of every
/// part, or of the recent parts only.
fn terms(store: &Store, recent_only: bool) -> Bits {
    let mut seen = Bits::default();
    for (p, table) in store.tables() {
        let v = table.by_subject();
        let stable = if recent_only { &[][..] } else { v.stable() };
        for part in std::iter::once(&v.recent()[..]).chain(stable.iter().map(|part| &part[..])) {
            if !part.is_empty() {
                seen.insert(p);
            }
            for &(s, o) in part {
                seen.insert(s);
                seen.insert(o);
            }
        }
    }
    seen
}

/// eq-ref on read: the representatives `r` of the terms of the store's
/// triples (any position) whose `r owl:sameAs r` the store does not hold,
/// in increasing order. Once expanded (`equality::expanded`), each stands
/// for `x owl:sameAs y` for every two members of its class. eq-ref's own
/// triples name `owl:sameAs`, so it is among them as soon as there is one.
/// A literal alone in its class is left out: its triple is not RDF.
pub(crate) fn same_as_itself(store: &Store, terms: &Dictionary) -> Vec<u32> {
    let canon = store.canon();
    let same = canon.find(vocab::OWL_SAME_AS.0);
    let stored = store.table(same).map(|t| t.by_subject());
    let mut all = self::terms(store, false);
    if all.iter().next().is_some() {
        all.insert(same);
    }
    all.iter()
        .filter(|&r| canon.members(r).len() > 1 || terms.kind(TermId(r)) != Kind::Literal)
        .filter(|&r| !stored.is_some_and(|v| v.contains(&(r, r))))
        .collect()
}

/// A set of term ids: a bitset as long as the largest needs.
#[derive(Debug, Default)]
struct Bits(Vec<u64>);

impl Bits {
    fn insert(&mut self, id: u32) {
        let word = id as usize / 64;
        if word >= self.0.len() {
            self.0.resize(word + 1, 0);
        }
        self.0[word] |= 1 << (id % 64);
    }

    /// The ids, in increasing order.
    fn iter(&self) -> impl Iterator<Item = u32> + '_ {
        self.0.iter().enumerate().flat_map(|(w, &word)| {
            let mut rest = word;
            std::iter::from_fn(move || {
                (rest != 0).then(|| {
                    let b = rest.trailing_zeros();
                    rest &= rest - 1;
                    w as u32 * 64 + b
                })
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::FACTS;
    use crate::rules::testing::{self, expand};
    use crate::term::Dictionary;
    use crate::{Format, Reasoner, Switch, Switches};
    use std::collections::BTreeSet;

    /// The run's inferred triples, eq-ref's `x owl:sameAs x` left out.
    fn without_eq_ref(run: &testing::Run) -> BTreeSet<String> {
        run.inferred()
            .into_iter()
            .filter(|t| {
                let terms: Vec<&str> = t.split(' ').collect();
                !(terms.len() == 3 && terms[1] == "<http://www.w3.org/2002/07/owl#sameAs>" && terms[0] == terms[2])
            })
            .collect()
    }

    fn facts() -> BTreeSet<String> {
        let dict = Dictionary::new();
        FACTS
            .iter()
            .map(|t| t.map(|id| dict.to_oxrdf(id).to_string()).join(" "))
            .collect()
    }

    #[test]
    fn scm_cls_scm_op_and_scm_dp_give_the_reflexive_and_bound_triples() {
        let run = testing::run(
            &["trivial"],
            "ex:C a owl:Class . ex:p a owl:ObjectProperty . ex:d a owl:DatatypeProperty . ex:x a ex:C .",
        );
        let derived: BTreeSet<String> = without_eq_ref(&run).difference(&facts()).cloned().collect();
        assert_eq!(
            derived,
            expand(
                "ex:C rdfs:subClassOf ex:C
                 ex:C owl:equivalentClass ex:C
                 ex:C rdfs:subClassOf owl:Thing
                 owl:Nothing rdfs:subClassOf ex:C
                 ex:p rdfs:subPropertyOf ex:p
                 ex:p owl:equivalentProperty ex:p
                 ex:d rdfs:subPropertyOf ex:d
                 ex:d owl:equivalentProperty ex:d"
            )
        );
    }

    /// prp-ap and dt-type1 hold for an empty input: the nine annotation
    /// properties, and every datatype of the map.
    #[test]
    fn prp_ap_and_dt_type1_are_facts() {
        let run = testing::run(&["trivial"], "");
        assert_eq!(without_eq_ref(&run), facts());
        for t in [
            "rdfs:label a owl:AnnotationProperty",
            "owl:incompatibleWith a owl:AnnotationProperty",
            "xsd:dateTimeStamp a rdfs:Datatype",
            "owl:real a rdfs:Datatype",
            "rdfs:Literal a rdfs:Datatype",
            "rdf:PlainLiteral a rdfs:Datatype",
        ] {
            assert!(run.has(t), "{t}");
        }
        assert_eq!(FACTS.len(), 9 + 35);
    }

    /// eq-ref on read: every IRI and blank node of the closure, in any
    /// position, is `owl:sameAs` itself; a literal's triple is not RDF.
    #[test]
    fn eq_ref_holds_of_every_term_of_the_closure() {
        let run = testing::run(&["trivial"], "ex:a ex:p _:b . _:b ex:q \"v\" .");
        for t in [
            "ex:a owl:sameAs ex:a",
            "ex:p owl:sameAs ex:p",
            "_:b owl:sameAs _:b",
            "ex:q owl:sameAs ex:q",
            "rdfs:label owl:sameAs rdfs:label",
            "rdf:type owl:sameAs rdf:type",
            "owl:sameAs owl:sameAs owl:sameAs",
        ] {
            assert!(run.has(t), "{t}");
        }
        assert!(!run.has("\"v\" owl:sameAs \"v\""));
        // A term the dictionary holds but no triple names is not in the
        // closure.
        assert!(!run.has("owl:hasKey owl:sameAs owl:hasKey"));
        // Nothing is stored for it.
        assert!(run.store.table(crate::term::vocab::OWL_SAME_AS.0).is_none());
    }

    /// A schema that reads `owl:sameAs` as a property sees eq-ref: the
    /// family stores it then, for every term, and for the terms of later
    /// runs.
    #[test]
    fn eq_ref_is_stored_where_a_rule_reads_it() {
        let mut run = testing::run(
            &["class", "property", "trivial"],
            "owl:sameAs rdfs:domain ex:C . ex:a ex:p ex:b .",
        );
        for t in [
            "ex:a a ex:C",
            "ex:b a ex:C",
            "ex:p a ex:C",
            "rdfs:domain a ex:C",
            "owl:Thing a ex:C",
        ] {
            assert!(run.has(t), "{t}");
        }
        run.add("ex:c ex:p \"v\" .");
        assert!(run.has("ex:c a ex:C"));
        assert!(run.has("\"v\" a ex:C"), "eq-ref holds of literals too, inside");
        // Through a restriction on owl:sameAs, and with owl:sameAs named
        // only through an equal term.
        let run = testing::run(
            &["class", "property", "equality", "trivial"],
            "ex:R owl:onProperty ex:same ; owl:someValuesFrom ex:D . ex:same owl:sameAs owl:sameAs . ex:x a ex:D .",
        );
        assert!(run.has("ex:x a ex:R"));
        // Without such a schema, nothing is stored.
        let run = testing::run(
            &["class", "property", "trivial"],
            "ex:p rdfs:domain ex:C . ex:a ex:p ex:b .",
        );
        assert!(run.store.table(crate::term::vocab::OWL_SAME_AS.0).is_none());
    }

    /// The reasoner adds eq-ref's triples to its reads and counts them in
    /// the summary, in full and incremental runs, with classes of equal
    /// terms; with `trivial` off, there are none.
    #[test]
    fn the_reasoner_reads_and_counts_eq_ref() {
        let prefixes = "@prefix ex: <http://example.org/> .
            @prefix owl: <http://www.w3.org/2002/07/owl#> .\n";
        let mut r = Reasoner::with_switches(Switches::complete());
        r.load_str(
            &format!("{prefixes}ex:a ex:p ex:b . ex:b owl:sameAs ex:c . ex:x ex:q \"v\" ."),
            Format::Turtle,
            None,
        )
        .unwrap();
        let counted = |r: &mut Reasoner| {
            let s = r.reason();
            let triples: Vec<_> = r.triples().collect();
            let unique: BTreeSet<String> = triples.iter().map(|t| t.to_string()).collect();
            assert_eq!(unique.len(), triples.len(), "a triple read twice");
            assert_eq!(s.base + s.inferred, triples.len());
            assert_eq!(s.inferred, r.inferred().count());
            unique
        };
        let same = |t: &str| {
            format!("<http://example.org/{t}> <http://www.w3.org/2002/07/owl#sameAs> <http://example.org/{t}>")
        };
        let closure = counted(&mut r);
        for t in ["a", "b", "c", "p", "x", "q"] {
            assert!(closure.contains(&same(t)), "{t}");
        }
        r.load_str(
            &format!("{prefixes}ex:y ex:p ex:a . ex:a owl:sameAs ex:z ."),
            Format::Turtle,
            None,
        )
        .unwrap();
        let closure = counted(&mut r);
        assert!(closure.contains(&same("y")) && closure.contains(&same("z")));
        r.set_switches(Switches::complete().off(Switch::Trivial));
        let closure = counted(&mut r);
        assert!(!closure.contains(&same("y")) && closure.contains(&same("z")));
    }
}
