//! The datatype family (switch `checks`): the OWL 2 RL datatype map
//! ([`map`]), which every datatype rule reads (dt-type2 in the `trivial`
//! group, dt-eq and dt-diff in `literal-values`), and dt-not-type.
//!
//! The rule derives no triple: it pushes a finding through
//! [`Context::report`], the literal then the datatype, as the public
//! `Diagnostic` documents.
//!
//! # dt-not-type
//!
//! Every datatype of the map is checked (`crates/reference/src/datatype.rs`
//! is the reference evaluator's map). A literal of a datatype outside the
//! map, or typed `rdfs:Literal`, has no known value and is never reported.
//! Two halves:
//!
//! - **Ill-typed** (a literal not in its own datatype's value space): both
//!   terms of every recent tuple of every predicate table, and the table's
//!   predicate (in generalized triples a literal can be a predicate, a
//!   table's key). Every term of the closure is in some tuple, and each
//!   tuple is recent in exactly one round, so this reads each tuple once
//!   over a run: linear in the closure, with no index and no state between
//!   calls. A pass over each literal once would need to remember which
//!   literals it has seen, which a stateless family cannot. A literal in
//!   many tuples is checked, and reported, once per tuple.
//! - **Typed** (`lit rdf:type dt` for a datatype `dt` of the map whose value
//!   space does not hold the literal's value, as prp-rng or a base triple
//!   states it): the recent part of `rdf:type` by object, at each datatype
//!   of the map. That index is built lazily ([`Round::by_object`]): the
//!   round after the first request, it holds every typing as recent, so
//!   nothing is lost.
//!
//! # Equality
//!
//! Every constant is read through the round's canon ([`Round::constant`],
//! ADR 0003). The rule looks inside the literal, so its findings name the
//! literal itself, not its representative.
//!
//! - **Ill-typed**: a literal is in some tuple, as it came, before a merge
//!   can make another term its representative (a fresh run reads the base
//!   as given, and a term new to an incremental run belongs to no class),
//!   and the reasoner keeps the findings for as long as it keeps the canon.
//!   So the representatives in the recent tuples are enough.
//! - **Typed**: `r rdf:type dt` stands for `m rdf:type dt` for every member
//!   `m` of `r`'s class, so each literal member is checked: those of a
//!   recent typing's class, and each literal that joined a class in the
//!   last step (the ids [`Round::moved`]) against the typings of its new
//!   representative.

#[cfg(test)]
mod edge_cases;
mod lexical;
pub(crate) mod map;
mod nat;

use super::Context;
use crate::engine::{Pair, Round, Variable, gallop};
use crate::term::{Dictionary, Kind, Term, TermId, vocab};
use map::{Datatype, Reading, value_of};

/// The datatype rules have no empty-body rule.
pub(crate) const FACTS: &[[TermId; 3]] = &[];

/// One round of dt-not-type.
pub(crate) fn rules(round: &mut Round<'_>, cx: &mut Context<'_>) {
    let ty = round.constant(vocab::RDF_TYPE.0);
    let types = round.by_object(ty);
    ill_typed(round, cx);
    moved_literals(round, ty, cx);
    if let Some(types) = types {
        typed_literals(round, types, cx);
    }
}

/// The pairs of a sorted slice whose first field is `key`.
fn with_key(pairs: &[Pair], key: u32) -> &[Pair] {
    let from = gallop(pairs, |t| t.0 < key);
    &from[..from.partition_point(|t| t.0 == key)]
}

/// dt-not-type, the ill-typed half (see the module doc): both terms of each
/// recent tuple, and each predicate with a recent tuple.
fn ill_typed(round: &Round<'_>, cx: &mut Context<'_>) {
    for (p, table) in round.store().tables() {
        let recent = table.by_subject().recent();
        if recent.is_empty() {
            continue;
        }
        check_own(cx, p);
        for &(s, o) in recent.iter() {
            check_own(cx, s);
            check_own(cx, o);
        }
    }
}

/// Reports `id` when it is an ill-typed literal.
#[inline]
fn check_own(cx: &mut Context<'_>, id: u32) {
    if cx.terms.kind(TermId(id)) != Kind::Literal {
        return;
    }
    let term = cx.terms.term(TermId(id));
    if let Term::Literal { datatype, .. } = term
        && matches!(value_of(term), Reading::IllTyped)
    {
        cx.report("dt-not-type", &[id, datatype.0]);
    }
}

/// dt-not-type, the typed half: a recent `x rdf:type dt` for a datatype
/// `dt` of the map, and each literal member of `x`'s class whose value
/// `dt`'s value space does not hold.
fn typed_literals(round: &Round<'_>, types: &Variable<Pair>, cx: &mut Context<'_>) {
    let canon = round.canon();
    for dt in Datatype::ALL {
        for &(_, x) in with_key(types.recent(), round.constant(dt.id().0)) {
            for m in canon.members(x).iter() {
                if in_value_space(cx.terms, TermId(m), dt) == Some(false) {
                    cx.report("dt-not-type", &[m, dt.id().0]);
                }
            }
        }
    }
}

/// dt-not-type, the typed half, for the literals that joined a class in the
/// last step: against every datatype of the map their new representative
/// is typed with (read by subject, so no object index is needed).
fn moved_literals(round: &Round<'_>, ty: u32, cx: &mut Context<'_>) {
    let moved = round.moved();
    if moved.is_empty() {
        return;
    }
    let Some(types) = round.by_subject(ty) else { return };
    let types = types.all();
    let datatypes = Datatype::ALL.map(|dt| (round.constant(dt.id().0), dt));
    for &m in moved {
        if cx.terms.kind(TermId(m)) != Kind::Literal {
            continue;
        }
        let r = round.canon().find(m);
        types.for_each_value(r, |class| {
            for &(c, dt) in &datatypes {
                if c == class && in_value_space(cx.terms, TermId(m), dt) == Some(false) {
                    cx.report("dt-not-type", &[m, dt.id().0]);
                }
            }
        });
    }
}

/// Whether the value of the term `id` is in the value space of `datatype`,
/// or `None` when its value is unknown (not a literal, or a literal of a
/// datatype outside the map). An ill-typed literal has no value, so it is
/// in no space.
fn in_value_space(terms: &Dictionary, id: TermId, datatype: Datatype) -> Option<bool> {
    match value_of(terms.term(id)) {
        Reading::Unknown => None,
        Reading::IllTyped => Some(false),
        Reading::Value(v) => Some(datatype.holds(&v)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Finding;
    use crate::rules::testing::{self, Run, expand};
    use std::collections::BTreeSet;

    fn run(turtle: &str) -> Run {
        testing::run(&["datatype"], turtle)
    }

    fn findings(turtle: &str) -> BTreeSet<String> {
        run(turtle).findings()
    }

    /// The id of a literal, interned if new.
    fn literal(run: &mut Run, lexical: &str, datatype: TermId) -> u32 {
        run.dict.intern_typed_literal(lexical, datatype).0
    }

    fn iri(run: &mut Run, iri: &str) -> u32 {
        run.dict.intern_iri(iri).0
    }

    #[test]
    fn dt_not_type_names_an_ill_typed_literal_and_its_datatype() {
        assert_eq!(
            findings(
                "ex:x ex:p \"abc\"^^xsd:integer , \"12\"^^xsd:integer , \"2147483648\"^^xsd:int ,
                    \"7\"^^xsd:int , \"<a>t</b>\"^^rdf:XMLLiteral , \"<a>t</a>\"^^rdf:XMLLiteral ,
                    \"plain\" , \"chat\"@fr , \"abc\"^^xsd:decimal , \"abc\"^^ex:dt ,
                    \"1.5\"^^xsd:decimal , \"1\"^^owl:real , \"x\"^^rdfs:Literal , \"TRUE\"^^xsd:boolean ,
                    \"2001-02-29T00:00:00\"^^xsd:dateTime , \"1/3\"^^owl:rational ."
            ),
            expand(
                "dt-not-type \"abc\"^^xsd:integer xsd:integer
                 dt-not-type \"2147483648\"^^xsd:int xsd:int
                 dt-not-type \"<a>t</b>\"^^rdf:XMLLiteral rdf:XMLLiteral
                 dt-not-type \"abc\"^^xsd:decimal xsd:decimal
                 dt-not-type \"1\"^^owl:real owl:real
                 dt-not-type \"TRUE\"^^xsd:boolean xsd:boolean
                 dt-not-type \"2001-02-29T00:00:00\"^^xsd:dateTime xsd:dateTime"
            )
        );
    }

    #[test]
    fn dt_not_type_for_a_literal_typed_with_a_datatype_of_the_map() {
        // A literal typed by a base triple (as prp-rng would type it): a
        // generalized triple, so inserted, not parsed.
        let mut run = run("ex:x ex:p \"abc\" , \"5\"^^xsd:int , \"9999999999\"^^xsd:integer , \"chat\"@fr .");
        let abc = literal(&mut run, "abc", vocab::XSD_STRING);
        let five = literal(&mut run, "5", vocab::XSD_INT);
        let big = literal(&mut run, "9999999999", vocab::XSD_INTEGER);
        let chat = run.dict.intern_lang_literal("chat", "fr").0;
        let ty = vocab::RDF_TYPE.0;
        for (s, dt) in [
            (abc, vocab::XSD_INTEGER),
            (abc, vocab::XSD_STRING),
            (five, vocab::XSD_INTEGER),
            (five, vocab::XSD_INT),
            (five, vocab::XSD_STRING),
            (big, vocab::XSD_INTEGER),
            (big, vocab::XSD_INT),
            (chat, vocab::RDF_LANG_STRING),
            (chat, vocab::XSD_STRING),
            (abc, vocab::XSD_DECIMAL),
            (five, vocab::XSD_DECIMAL),
            (five, vocab::XSD_DOUBLE),
            (five, vocab::XSD_UNSIGNED_BYTE),
            (five, vocab::XSD_NEGATIVE_INTEGER),
            (five, vocab::RDFS_LITERAL),
            (abc, vocab::XSD_TOKEN),
            (abc, vocab::RDF_PLAIN_LITERAL),
            (chat, vocab::RDF_PLAIN_LITERAL),
            (chat, vocab::RDFS_LITERAL),
        ] {
            run.store.insert(s, ty, dt.0);
        }
        run.add("");
        assert_eq!(
            run.findings(),
            expand(
                "dt-not-type \"abc\" xsd:integer
                 dt-not-type \"5\"^^xsd:int xsd:string
                 dt-not-type \"9999999999\"^^xsd:integer xsd:int
                 dt-not-type \"chat\"@fr xsd:string
                 dt-not-type \"abc\" xsd:decimal
                 dt-not-type \"5\"^^xsd:int xsd:double
                 dt-not-type \"5\"^^xsd:int xsd:negativeInteger"
            )
        );
    }

    #[test]
    fn an_iri_typed_with_a_datatype_is_not_a_literal_out_of_range() {
        // As prp-rng gives it for a property with a class or datatype range
        // and an IRI object: no dt-not-type.
        assert!(
            findings(
                "ex:p rdfs:range ex:C . ex:q rdfs:range xsd:integer .
                 ex:x ex:p ex:y ; ex:q ex:z .
                 ex:y a ex:C . ex:z a xsd:integer . _:b a xsd:string ."
            )
            .is_empty()
        );
        // With the property rules, once they type the objects.
        assert!(
            testing::run(
                &["property", "datatype"],
                "ex:p rdfs:range ex:C . ex:q rdfs:range xsd:integer . ex:x ex:p ex:y ; ex:q ex:z ."
            )
            .findings()
            .is_empty()
        );
    }

    #[test]
    fn dt_not_type_on_generalized_terms() {
        let mut run = run("ex:q owl:propertyDisjointWith ex:r .");
        let bad = literal(&mut run, "x", vocab::XSD_INT);
        let unsigned = literal(&mut run, "+", vocab::XSD_INTEGER);
        let five = literal(&mut run, "5", vocab::XSD_INTEGER);
        let [q, y] = ["q", "y"].map(|local| iri(&mut run, &format!("http://example.org/{local}")));
        for (x, pred, o) in [
            (five, q, five),
            // An ill-typed literal as a predicate only, and another as a subject.
            (y, bad, y),
            (unsigned, q, y),
        ] {
            run.store.insert(x, pred, o);
        }
        run.add("");
        assert_eq!(
            run.findings(),
            expand(
                "dt-not-type \"x\"^^xsd:int xsd:int
                 dt-not-type \"+\"^^xsd:integer xsd:integer"
            )
        );
    }

    /// Each half's premises in two parts: the finding appears only once the
    /// second part is added.
    #[test]
    fn dt_not_type_is_found_when_its_last_premise_arrives() {
        let mut ill = run("ex:x ex:p \"1\"^^xsd:integer .");
        assert!(ill.findings().is_empty());
        ill.add("ex:y ex:p \"a\"^^xsd:integer .");
        assert_eq!(ill.findings(), expand("dt-not-type \"a\"^^xsd:integer xsd:integer"));
        // The typed half, its typing added last.
        let mut run = run("ex:x ex:p \"abc\" .");
        let abc = literal(&mut run, "abc", vocab::XSD_STRING);
        run.add("");
        assert!(run.findings().is_empty());
        run.store.insert(abc, vocab::RDF_TYPE.0, vocab::XSD_INT.0);
        run.add("");
        assert_eq!(run.findings(), expand("dt-not-type \"abc\" xsd:int"));
    }

    #[test]
    fn a_family_reports_through_the_context() {
        // The findings are raw: duplicates are the reasoner's to remove.
        let run = run("ex:x ex:p \"a\"^^xsd:int . ex:y ex:p \"a\"^^xsd:int .");
        assert_eq!(run.findings.len(), 2);
        assert!(run.findings.iter().all(|f: &Finding| f.code == "dt-not-type"));
    }
}
