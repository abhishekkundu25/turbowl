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
//! Every datatype of the map is checked: the 32 OWL 2 RL supports
//! (`crates/reference/src/datatype.rs` is the reference evaluator's map). A
//! literal of a datatype outside the map (`owl:real`, `owl:rational` and
//! `rdf:langString` among them), or typed `rdfs:Literal`, has no known value
//! and is never reported; a language-tagged literal has its value, an
//! `rdf:PlainLiteral`'s. Two halves:
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
//!
//! # The literals of a closure
//!
//! dt-type2 (`rules/trivial.rs`) and dt-eq (`rules/literal_values.rs`)
//! hold of every literal with a value, once each. [`new_literals`] gives
//! each such literal to them in the round they first meet it.

#[cfg(test)]
mod edge_cases;
mod lexical;
pub(crate) mod map;

use super::{Context, Finding};
use crate::engine::{Pair, Round, Variable, gallop};
use crate::term::{Dictionary, Kind, Term, TermId, vocab};
use map::{Datatype, Reading, Value, value_of};

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
/// recent tuple, and each predicate with a recent tuple. A large recent
/// part is read in chunks ([`Round::split`]), whose findings join in order.
fn ill_typed(round: &mut Round<'_>, cx: &mut Context<'_>) {
    let terms = cx.terms;
    for (p, table) in round.store().tables() {
        let recent = table.by_subject().recent();
        if recent.is_empty() {
            continue;
        }
        check_own(terms, p, cx.findings);
        let found = round.split(recent, |_, chunk, found: &mut Vec<Finding>| {
            for &(s, o) in chunk {
                check_own(terms, s, found);
                check_own(terms, o, found);
            }
        });
        for mut chunk in found {
            cx.findings.append(&mut chunk);
        }
    }
}

/// Reports `id` to `found` when it is an ill-typed literal.
#[inline]
fn check_own(terms: &Dictionary, id: u32, found: &mut Vec<Finding>) {
    if terms.kind(TermId(id)) != Kind::Literal {
        return;
    }
    let term = terms.term(TermId(id));
    if let Term::Literal { datatype, .. } = term
        && matches!(value_of(term), Reading::IllTyped)
    {
        found.push(Finding {
            code: "dt-not-type",
            terms: vec![id, datatype.0],
        });
    }
}

/// dt-not-type, the typed half: a recent `x rdf:type dt` for a datatype
/// `dt` of the map, and each literal member of `x`'s class whose value
/// `dt`'s value space does not hold.
fn typed_literals(round: &Round<'_>, types: &Variable<Pair>, cx: &mut Context<'_>) {
    let canon = round.canon();
    for dt in Datatype::OWL_2_RL {
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
    let datatypes = Datatype::OWL_2_RL.map(|dt| (round.constant(dt.id().0), dt));
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

/// The literals the W3C rules name: the cardinalities `"0"` and `"1"` of
/// `xsd:nonNegativeInteger` (cls-maxc1 to cls-maxqc4). They are literals of
/// the rule set, so the rules over every literal (dt-type2, dt-eq, dt-diff)
/// range over them too, whether or not the closure names them: with dt-eq,
/// `"1"^^xsd:integer` is the same as the cardinality the rules read.
pub(crate) const RULE_LITERALS: [&str; 2] = ["0", "1"];

/// Interns [`RULE_LITERALS`] (the reasoner does, with every new dictionary).
pub(crate) fn intern_rule_literals(terms: &mut Dictionary) {
    for lexical in RULE_LITERALS {
        terms.intern_typed_literal(lexical, vocab::XSD_NON_NEGATIVE_INTEGER);
    }
}

/// The ids of [`RULE_LITERALS`] the dictionary holds.
pub(crate) fn rule_literals(terms: &Dictionary) -> impl Iterator<Item = u32> + '_ {
    RULE_LITERALS.into_iter().filter_map(|lexical| {
        let literal = oxrdf::LiteralRef::new_typed_literal(lexical, oxrdf::vocab::xsd::NON_NEGATIVE_INTEGER);
        terms.get_term(literal.into(), crate::term::DocId::NONE).map(|id| id.0)
    })
}

/// The literals with a value that a rule over every literal (dt-type2,
/// dt-eq) has not handled yet, each once, with its value.
///
/// No rule makes a literal, so every literal of the closure is a base
/// literal, and it is in a tuple, as itself, in the first round in which
/// the store holds it: the store's first round (a full run, or a restart)
/// or the first round of the incremental run that adds it. The literals
/// the rules name ([`RULE_LITERALS`]) are interned with the dictionary. So:
///
/// - In the store's first round, every tuple is recent, and every literal
///   with a value is new: those of the tuples, the literals the rules name,
///   and the literal members of the classes the canon kept from before a
///   restart, which their representatives hide.
/// - In a later round, a new literal is one of a recent tuple's, alone in
///   its class (a term new to an incremental run belongs to no class),
///   that `handled` does not hold of. A rule marks each literal it handles
///   with a triple of its own (dt-type2's `lt rdf:type rdfs:Literal`,
///   dt-eq's `lt owl:sameAs lt`), which the store holds from the next round
///   on and keeps on the literal's representative through every merge.
///
/// A later round reads each recent tuple once, as the ill-typed half of
/// dt-not-type does; the store's first round reads the whole store once.
pub(crate) fn new_literals<'a>(
    round: &Round<'_>,
    terms: &'a Dictionary,
    handled: impl Fn(u32) -> bool,
) -> Vec<(u32, Value<'a>)> {
    let literal = |id: u32| terms.kind(TermId(id)) == Kind::Literal;
    let first = round.number() == 1;
    let mut found = Vec::new();
    for (_, table) in round.store().tables() {
        for &(s, o) in table.by_subject().recent().iter() {
            for t in [s, o] {
                if literal(t) && (first || !handled(t)) {
                    found.push(t);
                }
            }
        }
    }
    if first {
        found.extend(rule_literals(terms));
        for (_, members) in round.canon().classes() {
            found.extend(members.iter().copied().filter(|&m| literal(m)));
        }
    }
    found.sort_unstable();
    found.dedup();
    found
        .into_iter()
        .filter_map(|id| match value_of(terms.term(TermId(id))) {
            Reading::Value(v) => Some((id, v)),
            _ => None,
        })
        .collect()
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

    /// `owl:real`, `owl:rational` and `rdf:langString` are outside the map
    /// (OWL 2 RL does not support them): their literals are never ill-typed.
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
            // Outside the map: never checked.
            (abc, vocab::RDF_LANG_STRING),
            (abc, vocab::OWL_REAL),
            (five, vocab::OWL_RATIONAL),
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
