//! The rules that conclude `false` (ticket 09): cax-dw, prp-pdw, prp-asyp,
//! prp-irp, cls-nothing2, cls-com, and dt-not-type for the checked
//! datatypes. They derive no triple: each pushes a finding through
//! [`Context::report`], with its terms in the order the public `Diagnostic`
//! documents (the oracle crate's `Finding` and the reference evaluator's
//! `Diagnostics` give the same orders).
//!
//! # Semi-naive
//!
//! Every finding is reported in the round its last premise arrives: each
//! rule joins the recent part of one premise with all of the others, for
//! every premise. A finding may be reported more than once (two premises
//! recent in the same round, or a literal in several tuples); the reasoner
//! deduplicates.
//!
//! The rules read `rdf:type`, `owl:disjointWith` and `owl:complementOf` by
//! object, an index the engine builds lazily ([`Round::by_object`]): the
//! first round that asks gets `None`, and the round after, the index holds
//! every pair as recent. A rule that skips a round for want of an index
//! loses nothing, because every one of its findings has a premise in that
//! index and the rule reads that relation's recent part from the index
//! (cax-dw and cls-com read their axioms by object only).
//!
//! # dt-not-type
//!
//! The checked datatypes are `xsd:string`, `xsd:integer`, `xsd:int`,
//! `rdf:langString` and `rdf:XMLLiteral`, with the value spaces of the
//! reference evaluator (`crates/reference/src/datatype.rs`), ported here.
//! A literal of any other datatype is never reported. Two halves:
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
//! - **Typed** (`lit rdf:type dt` for a checked `dt` whose value space does
//!   not hold the literal's value, as prp-rng or a base triple states it):
//!   the recent part of `rdf:type` by object, at each checked datatype.

use super::Context;
use crate::engine::{Pair, Round, Variable, gallop};
use crate::term::{Dictionary, Kind, Term, TermId, vocab};

/// The diagnostic rules have no empty-body rule.
pub(crate) const FACTS: &[[TermId; 3]] = &[];

/// The datatypes dt-not-type checks.
const CHECKED: [TermId; 5] = [
    vocab::XSD_STRING,
    vocab::XSD_INTEGER,
    vocab::XSD_INT,
    vocab::RDF_LANG_STRING,
    vocab::RDF_XML_LITERAL,
];

/// One round of the diagnostic rules.
pub(crate) fn rules(round: &mut Round<'_>, cx: &mut Context<'_>) {
    // Every object index is asked for before any rule returns early, so
    // that all the missing ones are built for the next round together.
    let types = round.by_object(vocab::RDF_TYPE.0);
    let disjoint = round.by_object(vocab::OWL_DISJOINT_WITH.0);
    let complement = round.by_object(vocab::OWL_COMPLEMENT_OF.0);

    ill_typed(round, cx);
    property_disjoint(round, cx);
    let Some(types) = types else { return };
    if let Some(axioms) = disjoint {
        class_pairs(types, axioms, |c1, c2, x| cx.report("cax-dw", &[x, c1, c2]));
    }
    if let Some(axioms) = complement {
        class_pairs(types, axioms, |c1, c2, x| cx.report("cls-com", &[c1, c2, x]));
    }
    for &(_, x) in with_key(types.recent(), vocab::OWL_NOTHING.0) {
        cx.report("cls-nothing2", &[x]);
    }
    asymmetric(round, types, cx);
    irreflexive(round, types, cx);
    typed_literals(types, cx);
}

/// The pairs of a sorted slice whose first field is `key`.
fn with_key(pairs: &[Pair], key: u32) -> &[Pair] {
    let from = gallop(pairs, |t| t.0 < key);
    &from[..from.partition_point(|t| t.0 == key)]
}

/// cax-dw and cls-com: `c1 R c2`, `x rdf:type c1`, `x rdf:type c2` gives
/// `found(c1, c2, x)`. `types` is `rdf:type` by object, `(class, x)`, and
/// `axioms` is `R` by object, `(c2, c1)` (see the module doc). Each case
/// intersects two classes' instances, sorted runs of `types`, by merging:
///
/// - a recent axiom: all instances of `c1` with all of `c2`;
/// - an earlier axiom: the recent instances of `c1` with all of `c2`, and
///   the earlier instances of `c1` with the recent ones of `c2`.
///
/// The axioms are schema: reading the earlier ones each round costs their
/// number, not the data's.
fn class_pairs(types: &Variable<Pair>, axioms: &Variable<Pair>, mut found: impl FnMut(u32, u32, u32)) {
    let individual = |t: &Pair| t.1;
    for &(c2, c1) in axioms.recent().iter() {
        for a in types.parts() {
            for b in types.parts() {
                common(with_key(a, c1), with_key(b, c2), individual, |(_, x)| found(c1, c2, x));
            }
        }
    }
    let recent = types.recent();
    if recent.is_empty() {
        return;
    }
    for part in axioms.stable() {
        for &(c2, c1) in part.iter() {
            let r1 = with_key(recent, c1);
            if !r1.is_empty() {
                for b in types.parts() {
                    common(r1, with_key(b, c2), individual, |(_, x)| found(c1, c2, x));
                }
            }
            let r2 = with_key(recent, c2);
            if !r2.is_empty() {
                for a in types.stable() {
                    common(with_key(a, c1), r2, individual, |(_, x)| found(c1, c2, x));
                }
            }
        }
    }
}

/// Calls `f(pair)` with each pair of `a` whose key some pair of `b` has.
/// Both are sorted by `key` without repeats; the merge gallops over gaps.
fn common<K: Ord>(mut a: &[Pair], mut b: &[Pair], key: impl Fn(&Pair) -> K, mut f: impl FnMut(Pair)) {
    while let (Some(x), Some(y)) = (a.first(), b.first()) {
        let (kx, ky) = (key(x), key(y));
        match kx.cmp(&ky) {
            std::cmp::Ordering::Less => a = gallop(a, |t| key(t) < ky),
            std::cmp::Ordering::Greater => b = gallop(b, |t| key(t) < kx),
            std::cmp::Ordering::Equal => {
                f(*x);
                a = &a[1..];
                b = &b[1..];
            }
        }
    }
}

/// prp-pdw: `p1 owl:propertyDisjointWith p2`, `x p1 y`, `x p2 y` gives
/// (p1, p2, x, y). A recent axiom intersects the two tables whole; an
/// earlier one intersects each table's recent part with the other table
/// (recent with recent once). The axioms are schema: reading the earlier
/// ones each round costs their number, not the data's.
fn property_disjoint(round: &Round<'_>, cx: &mut Context<'_>) {
    let Some(axioms) = round.by_subject(vocab::OWL_PROPERTY_DISJOINT_WITH.0) else {
        return;
    };
    let tables = |p1: u32, p2: u32| Some((round.by_subject(p1)?, round.by_subject(p2)?));
    for &(p1, p2) in axioms.recent().iter() {
        let Some((t1, t2)) = tables(p1, p2) else { continue };
        for a in t1.parts() {
            for b in t2.parts() {
                common(a, b, |&t| t, |(x, y)| cx.report("prp-pdw", &[p1, p2, x, y]));
            }
        }
    }
    for part in axioms.stable() {
        for &(p1, p2) in part.iter() {
            let Some((t1, t2)) = tables(p1, p2) else { continue };
            let (r1, r2) = (t1.recent(), t2.recent());
            if !r1.is_empty() {
                for b in t2.parts() {
                    common(r1, b, |&t| t, |(x, y)| cx.report("prp-pdw", &[p1, p2, x, y]));
                }
            }
            if !r2.is_empty() {
                for a in t1.stable() {
                    common(a, r2, |&t| t, |(x, y)| cx.report("prp-pdw", &[p1, p2, x, y]));
                }
            }
        }
    }
}

/// Calls `f(p, table, whole)` for each property `p` typed `class` that has
/// a table: with `whole` when the typing is recent (the rule reads the
/// whole table), without it when it is earlier and the table has a recent
/// part (the rule reads that part).
fn properties_of<'a>(
    round: &Round<'a>,
    types: &'a Variable<Pair>,
    class: TermId,
    mut f: impl FnMut(u32, &'a Variable<Pair>, bool),
) {
    for &(_, p) in with_key(types.recent(), class.0) {
        if let Some(table) = round.by_subject(p) {
            f(p, table, true);
        }
    }
    for part in types.stable() {
        for &(_, p) in with_key(part, class.0) {
            if let Some(table) = round.by_subject(p).filter(|t| !t.recent().is_empty()) {
                f(p, table, false);
            }
        }
    }
}

/// prp-asyp: `p rdf:type owl:AsymmetricProperty`, `x p y`, `y p x` gives
/// (p, x, y). A recent pair `(x, y)` whose reverse is held is the last
/// premise of two findings, (p, x, y) and (p, y, x).
fn asymmetric(round: &Round<'_>, types: &Variable<Pair>, cx: &mut Context<'_>) {
    properties_of(round, types, vocab::OWL_ASYMMETRIC_PROPERTY, |p, table, whole| {
        if whole {
            for part in table.parts() {
                for &(x, y) in part {
                    if table.contains(&(y, x)) {
                        cx.report("prp-asyp", &[p, x, y]);
                    }
                }
            }
        } else {
            for &(x, y) in table.recent().iter() {
                if table.contains(&(y, x)) {
                    cx.report("prp-asyp", &[p, x, y]);
                    if x != y {
                        cx.report("prp-asyp", &[p, y, x]);
                    }
                }
            }
        }
    });
}

/// prp-irp: `p rdf:type owl:IrreflexiveProperty`, `x p x` gives (p, x).
fn irreflexive(round: &Round<'_>, types: &Variable<Pair>, cx: &mut Context<'_>) {
    properties_of(round, types, vocab::OWL_IRREFLEXIVE_PROPERTY, |p, table, whole| {
        let mut check = |part: &[Pair]| {
            for &(x, y) in part {
                if x == y {
                    cx.report("prp-irp", &[p, x]);
                }
            }
        };
        if whole {
            table.parts().for_each(&mut check);
        } else {
            check(table.recent());
        }
    });
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

/// Reports `id` when it is a literal outside its own datatype's value space.
#[inline]
fn check_own(cx: &mut Context<'_>, id: u32) {
    if cx.terms.kind(TermId(id)) != Kind::Literal {
        return;
    }
    if let Term::Literal {
        lexical,
        datatype,
        language,
    } = cx.terms.term(TermId(id))
        && well_formed(lexical, datatype, language) == Some(false)
    {
        cx.report("dt-not-type", &[id, datatype.0]);
    }
}

/// dt-not-type, the typed half: a recent `lit rdf:type dt` for a checked
/// `dt` whose value space does not hold `lit`.
fn typed_literals(types: &Variable<Pair>, cx: &mut Context<'_>) {
    for dt in CHECKED {
        for &(_, x) in with_key(types.recent(), dt.0) {
            if in_value_space(cx.terms, TermId(x), dt) == Some(false) {
                cx.report("dt-not-type", &[x, dt.0]);
            }
        }
    }
}

/// Whether a literal is well-formed for its own datatype, or `None` when
/// that datatype is not checked.
fn well_formed(lexical: &str, datatype: TermId, language: Option<&str>) -> Option<bool> {
    Some(match datatype {
        vocab::XSD_STRING => true,
        vocab::XSD_INTEGER => integer(lexical),
        vocab::XSD_INT => int(lexical),
        vocab::RDF_LANG_STRING => language.is_some(),
        vocab::RDF_XML_LITERAL => xml(lexical),
        _ => return None,
    })
}

/// Whether the value of the term `id` is in the value space of `datatype`,
/// or `None` when `id` is not a literal of a checked datatype (its value is
/// unknown here) or `datatype` is not checked (its space is unknown).
///
/// The value spaces, as XML Schema and RDF 1.1 define them: an integer (of
/// `xsd:integer` or `xsd:int`) is in `xsd:integer`, and in `xsd:int` when
/// it fits 32 bits; strings, language-tagged strings and XML values are
/// each a space of their own. An ill-typed literal has no value, so it is
/// in no space.
fn in_value_space(terms: &Dictionary, id: TermId, datatype: TermId) -> Option<bool> {
    let Term::Literal {
        lexical,
        datatype: own,
        language,
    } = terms.term(id)
    else {
        return None;
    };
    if !CHECKED.contains(&datatype) {
        return None;
    }
    if !well_formed(lexical, own, language)? {
        return Some(false);
    }
    Some(match (own, datatype) {
        (vocab::XSD_INTEGER | vocab::XSD_INT, vocab::XSD_INTEGER) => true,
        (vocab::XSD_INTEGER | vocab::XSD_INT, vocab::XSD_INT) => int(lexical),
        (own, datatype) => own == datatype,
    })
}

/// An optional sign, then one or more ASCII digits.
fn integer(lexical: &str) -> bool {
    let digits = lexical.strip_prefix(['+', '-']).unwrap_or(lexical);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

/// An integer from -2^31 to 2^31 - 1 (leading zeros allowed).
fn int(lexical: &str) -> bool {
    if !integer(lexical) {
        return false;
    }
    let (negative, digits) = match lexical.strip_prefix('-') {
        Some(digits) => (true, digits),
        None => (false, lexical.strip_prefix('+').unwrap_or(lexical)),
    };
    let digits = digits.trim_start_matches('0');
    // 2^31 has 10 digits: more significant digits are out of range, and
    // 10 fit a u64.
    if digits.len() > 10 {
        return false;
    }
    let magnitude = digits.bytes().fold(0u64, |m, b| m * 10 + u64::from(b - b'0'));
    if negative {
        magnitude <= 1 << 31
    } else {
        magnitude < 1 << 31
    }
}

/// Balanced XML content, checked for its tags only: every end tag closes
/// the innermost open element, every markup construct is closed by its
/// `>`, and no element is left open. Plain text is content too. Entities
/// and attribute syntax are not checked.
fn xml(lexical: &str) -> bool {
    let mut open = Names::default();
    let mut rest = lexical;
    while let Some(start) = rest.find('<') {
        rest = &rest[start..];
        let (end_marker, skip) = if rest.starts_with("<!--") {
            ("-->", 4)
        } else if rest.starts_with("<![CDATA[") {
            ("]]>", 9)
        } else if rest.starts_with("<?") {
            ("?>", 2)
        } else {
            (">", 1)
        };
        let Some(len) = rest[skip..].find(end_marker) else {
            return false;
        };
        let inner = &rest[1..skip + len];
        rest = &rest[skip + len + end_marker.len()..];
        if skip != 1 || inner.starts_with('!') {
            continue;
        }
        if let Some(name) = inner.strip_prefix('/') {
            if open.pop() != Some(name.trim_end()) {
                return false;
            }
        } else if !inner.ends_with('/') {
            open.push(inner.split(char::is_whitespace).next().unwrap_or(""));
        }
    }
    open.is_empty()
}

/// The open elements' names: the first [`Names::INLINE`] in place, deeper
/// ones in a vector, so the usual XML literal allocates nothing.
#[derive(Debug, Default)]
struct Names<'a> {
    inline: [&'a str; Names::INLINE],
    len: usize,
    deeper: Vec<&'a str>,
}

impl<'a> Names<'a> {
    const INLINE: usize = 16;

    fn push(&mut self, name: &'a str) {
        if self.len < Self::INLINE {
            self.inline[self.len] = name;
            self.len += 1;
        } else {
            self.deeper.push(name);
        }
    }

    fn pop(&mut self) -> Option<&'a str> {
        self.deeper.pop().or_else(|| {
            self.len = self.len.checked_sub(1)?;
            Some(self.inline[self.len])
        })
    }

    fn is_empty(&self) -> bool {
        self.len == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{Store, join};
    use crate::rules::testing::{self, Run, expand};
    use crate::rules::{Family, Finding};
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
    fn cax_dw_names_the_individual_then_the_classes_as_stated() {
        assert_eq!(
            findings("ex:A owl:disjointWith ex:B . ex:x a ex:A , ex:B . ex:y a ex:A ."),
            expand("cax-dw ex:x ex:A ex:B")
        );
        // A class disjoint with itself has no instance.
        assert_eq!(
            findings("ex:A owl:disjointWith ex:A . ex:x a ex:A ."),
            expand("cax-dw ex:x ex:A ex:A")
        );
    }

    #[test]
    fn prp_pdw_names_the_properties_as_stated_then_subject_and_object() {
        assert_eq!(
            findings(
                "ex:p owl:propertyDisjointWith ex:q .
                 ex:x ex:p ex:y ; ex:q ex:y .
                 ex:x ex:p ex:z . ex:z ex:q ex:x ."
            ),
            expand("prp-pdw ex:p ex:q ex:x ex:y")
        );
    }

    #[test]
    fn cls_nothing2_names_the_individual() {
        assert_eq!(
            findings("ex:x a owl:Nothing . ex:y a ex:C ."),
            expand("cls-nothing2 ex:x")
        );
    }

    #[test]
    fn prp_asyp_names_the_property_subject_and_object_both_ways() {
        assert_eq!(
            findings(
                "ex:p a owl:AsymmetricProperty .
                 ex:x ex:p ex:y . ex:y ex:p ex:x . ex:y ex:p ex:z .
                 ex:s ex:p ex:s .
                 ex:a ex:q ex:b . ex:b ex:q ex:a ."
            ),
            expand(
                "prp-asyp ex:p ex:x ex:y
                 prp-asyp ex:p ex:y ex:x
                 prp-asyp ex:p ex:s ex:s"
            )
        );
    }

    #[test]
    fn prp_irp_names_the_property_and_the_individual() {
        assert_eq!(
            findings(
                "ex:p a owl:IrreflexiveProperty .
                 ex:x ex:p ex:x . ex:x ex:p ex:y . ex:z ex:q ex:z ."
            ),
            expand("prp-irp ex:p ex:x")
        );
    }

    #[test]
    fn cls_com_names_the_classes_as_stated_then_the_individual() {
        assert_eq!(
            findings("ex:A owl:complementOf ex:B . ex:x a ex:A , ex:B . ex:y a ex:B ."),
            expand("cls-com ex:A ex:B ex:x")
        );
    }

    #[test]
    fn dt_not_type_names_an_ill_typed_literal_and_its_datatype() {
        assert_eq!(
            findings(
                "ex:x ex:p \"abc\"^^xsd:integer , \"12\"^^xsd:integer , \"2147483648\"^^xsd:int ,
                    \"7\"^^xsd:int , \"<a>t</b>\"^^rdf:XMLLiteral , \"<a>t</a>\"^^rdf:XMLLiteral ,
                    \"plain\" , \"chat\"@fr , \"abc\"^^xsd:decimal , \"abc\"^^ex:dt ."
            ),
            expand(
                "dt-not-type \"abc\"^^xsd:integer xsd:integer
                 dt-not-type \"2147483648\"^^xsd:int xsd:int
                 dt-not-type \"<a>t</b>\"^^rdf:XMLLiteral rdf:XMLLiteral"
            )
        );
    }

    #[test]
    fn dt_not_type_for_a_literal_typed_with_a_checked_datatype() {
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
                 dt-not-type \"chat\"@fr xsd:string"
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
    fn findings_on_generalized_terms() {
        let mut run = run("ex:A owl:disjointWith ex:B . ex:C owl:complementOf ex:D .
             ex:p a owl:IrreflexiveProperty , owl:AsymmetricProperty .
             ex:q owl:propertyDisjointWith ex:r .
             _:b1 a ex:A , ex:B .");
        let ty = vocab::RDF_TYPE.0;
        let five = literal(&mut run, "5", vocab::XSD_INTEGER);
        let bad = literal(&mut run, "x", vocab::XSD_INT);
        let unsigned = literal(&mut run, "+", vocab::XSD_INTEGER);
        let s = literal(&mut run, "s", vocab::XSD_STRING);
        let [a, b, c, d, p, q, r, y] =
            ["A", "B", "C", "D", "p", "q", "r", "y"].map(|local| iri(&mut run, &format!("http://example.org/{local}")));
        for (x, pred, o) in [
            (five, ty, a),
            (five, ty, b),
            (s, ty, c),
            (s, ty, d),
            (s, ty, vocab::OWL_NOTHING.0),
            (five, p, five),
            (five, p, s),
            (s, p, five),
            (s, q, five),
            (s, r, five),
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
                "cax-dw _:b1 ex:A ex:B
                 cax-dw \"5\"^^xsd:integer ex:A ex:B
                 cls-com ex:C ex:D \"s\"
                 cls-nothing2 \"s\"
                 prp-irp ex:p \"5\"^^xsd:integer
                 prp-asyp ex:p \"5\"^^xsd:integer \"5\"^^xsd:integer
                 prp-asyp ex:p \"5\"^^xsd:integer \"s\"
                 prp-asyp ex:p \"s\" \"5\"^^xsd:integer
                 prp-pdw ex:q ex:r \"s\" \"5\"^^xsd:integer
                 dt-not-type \"x\"^^xsd:int xsd:int
                 dt-not-type \"+\"^^xsd:integer xsd:integer"
            )
        );
    }

    /// Each finding's premises in two parts: the finding appears only once
    /// the second part is added, whichever part comes last.
    #[test]
    fn a_finding_is_found_when_its_last_premise_arrives() {
        let cases = [
            (
                "ex:A owl:disjointWith ex:B . ex:x a ex:A .",
                "ex:x a ex:B .",
                "cax-dw ex:x ex:A ex:B",
            ),
            (
                "ex:x a ex:A , ex:B .",
                "ex:A owl:disjointWith ex:B .",
                "cax-dw ex:x ex:A ex:B",
            ),
            (
                "ex:A owl:disjointWith ex:B . ex:x a ex:B .",
                "ex:x a ex:A .",
                "cax-dw ex:x ex:A ex:B",
            ),
            (
                "ex:A owl:complementOf ex:B . ex:x a ex:B .",
                "ex:x a ex:A .",
                "cls-com ex:A ex:B ex:x",
            ),
            (
                "ex:x a ex:A , ex:B .",
                "ex:A owl:complementOf ex:B .",
                "cls-com ex:A ex:B ex:x",
            ),
            (
                "ex:p owl:propertyDisjointWith ex:q . ex:x ex:p ex:y .",
                "ex:x ex:q ex:y .",
                "prp-pdw ex:p ex:q ex:x ex:y",
            ),
            (
                "ex:p owl:propertyDisjointWith ex:q . ex:x ex:q ex:y .",
                "ex:x ex:p ex:y .",
                "prp-pdw ex:p ex:q ex:x ex:y",
            ),
            (
                "ex:x ex:p ex:y ; ex:q ex:y .",
                "ex:p owl:propertyDisjointWith ex:q .",
                "prp-pdw ex:p ex:q ex:x ex:y",
            ),
            (
                "ex:p a owl:AsymmetricProperty . ex:x ex:p ex:y .",
                "ex:y ex:p ex:x .",
                "prp-asyp ex:p ex:x ex:y\nprp-asyp ex:p ex:y ex:x",
            ),
            (
                "ex:x ex:p ex:y . ex:y ex:p ex:x .",
                "ex:p a owl:AsymmetricProperty .",
                "prp-asyp ex:p ex:x ex:y\nprp-asyp ex:p ex:y ex:x",
            ),
            (
                "ex:p a owl:IrreflexiveProperty . ex:x ex:p ex:y .",
                "ex:x ex:p ex:x .",
                "prp-irp ex:p ex:x",
            ),
            (
                "ex:x ex:p ex:x .",
                "ex:p a owl:IrreflexiveProperty .",
                "prp-irp ex:p ex:x",
            ),
            ("ex:x a ex:C .", "ex:x a owl:Nothing .", "cls-nothing2 ex:x"),
            (
                "ex:x ex:p \"1\"^^xsd:integer .",
                "ex:y ex:p \"a\"^^xsd:integer .",
                "dt-not-type \"a\"^^xsd:integer xsd:integer",
            ),
        ];
        for (first, second, expected) in cases {
            let mut run = run(first);
            assert!(run.findings().is_empty(), "{first} alone: {:?}", run.findings());
            run.add(second);
            assert_eq!(run.findings(), expand(expected), "{first} then {second}");
        }
        // The typed half of dt-not-type, its typing added last.
        let mut run = run("ex:x ex:p \"abc\" .");
        let abc = literal(&mut run, "abc", vocab::XSD_STRING);
        run.add("");
        assert!(run.findings().is_empty());
        run.store.insert(abc, vocab::RDF_TYPE.0, vocab::XSD_INT.0);
        run.add("");
        assert_eq!(run.findings(), expand("dt-not-type \"abc\" xsd:int"));
    }

    /// `x ex:next y` and `y rdf:type c` give `x rdf:type c`: types that
    /// arrive one round after another, after the object indexes exist.
    fn backwards(round: &mut Round<'_>, cx: &mut Context<'_>) {
        let next = cx.terms.get_iri("http://example.org/next").expect("in the input").0;
        let (Some(next), Some(types)) = (round.by_object(next), round.by_subject(vocab::RDF_TYPE.0)) else {
            return;
        };
        let mut derived = Vec::new();
        join(next, types, |_, x, c| derived.push((x, c)));
        for (x, c) in derived {
            round.emit(x, vocab::RDF_TYPE.0, c);
        }
    }

    static BACKWARDS: Family = Family {
        name: "backwards",
        rules: backwards,
        facts: &[],
    };

    #[test]
    fn premises_derived_in_later_rounds_are_joined() {
        let run = testing::run_families(
            vec![&BACKWARDS, testing::family("datatype")],
            "ex:A owl:disjointWith ex:B . ex:C owl:complementOf ex:D .
             ex:x0 ex:next ex:x1 . ex:x1 ex:next ex:x2 . ex:x2 ex:next ex:x3 . ex:x3 ex:next ex:x4 .
             ex:x4 a ex:A , ex:C , owl:Nothing . ex:x0 a ex:B . ex:x1 a ex:D .",
        );
        assert!(run.rounds >= 5, "{} rounds", run.rounds);
        let mut expected = expand(
            "cax-dw ex:x0 ex:A ex:B
             cls-com ex:C ex:D ex:x1
             cls-com ex:C ex:D ex:x0",
        );
        for i in 0..5 {
            expected.extend(expand(&format!("cls-nothing2 ex:x{i}")));
        }
        assert_eq!(run.findings(), expected);
    }

    fn own(lexical: &str, datatype: TermId) -> Option<bool> {
        well_formed(lexical, datatype, None)
    }

    #[test]
    fn integers_as_the_reference_reads_them() {
        for (value, ok) in [
            ("+5", true),
            ("-7", true),
            ("42", true),
            ("007", true),
            ("0", true),
            ("-0", true),
            ("99999999999999999999999", true),
            (" 42", false),
            ("42 ", false),
            ("4 2", false),
            ("abc", false),
            ("", false),
            ("+", false),
            ("-", false),
            ("+-5", false),
            ("1.0", false),
            ("1e3", false),
            ("٣", false),
        ] {
            assert_eq!(own(value, vocab::XSD_INTEGER), Some(ok), "integer {value:?}");
        }
        for (value, ok) in [
            ("-7", true),
            ("+7", true),
            ("2147483647", true),
            ("+2147483647", true),
            ("-2147483648", true),
            ("0002147483647", true),
            ("-0002147483648", true),
            ("-0", true),
            ("2147483648", false),
            ("-2147483649", false),
            ("4294967296", false),
            ("99999999999", false),
            ("99999999999999999999999", false),
            (" 42", false),
            ("", false),
            ("abc", false),
        ] {
            assert_eq!(own(value, vocab::XSD_INT), Some(ok), "int {value:?}");
        }
    }

    #[test]
    fn strings_language_strings_and_unchecked_datatypes() {
        for value in ["", " x ", "abc", "<a>"] {
            assert_eq!(own(value, vocab::XSD_STRING), Some(true), "{value:?}");
        }
        assert_eq!(well_formed("chat", vocab::RDF_LANG_STRING, Some("fr")), Some(true));
        assert_eq!(well_formed("chat", vocab::RDF_LANG_STRING, None), Some(false));
        for datatype in [
            vocab::XSD_DECIMAL,
            vocab::XSD_BOOLEAN,
            vocab::XSD_LONG,
            vocab::RDF_PLAIN_LITERAL,
        ] {
            assert_eq!(own("abc", datatype), None);
        }
    }

    #[test]
    fn xml_literals_are_checked_for_their_tags() {
        for (value, ok) in [
            ("<a>t</a>", true),
            ("plain text", true),
            ("", true),
            ("<a><b/></a>", true),
            ("<a x=\"1\">t</a>", true),
            ("<a >t</a >", true),
            ("<!-- c --><a/>", true),
            ("<![CDATA[<a>]]>", true),
            ("<?pi x?><a/>", true),
            ("<!DOCTYPE a><a/>", true),
            ("<a>", false),
            ("<a>t</b>", false),
            ("</a>", false),
            ("<a", false),
            ("<!-- c", false),
            ("<a><b></a></b>", false),
        ] {
            assert_eq!(own(value, vocab::RDF_XML_LITERAL), Some(ok), "{value:?}");
        }
        // Deeper than the names kept in place.
        let deep = |n: usize| "<e>".repeat(n) + &"</e>".repeat(n);
        assert_eq!(own(&deep(40), vocab::RDF_XML_LITERAL), Some(true));
        assert_eq!(own(&(deep(40) + "</e>"), vocab::RDF_XML_LITERAL), Some(false));
        assert_eq!(own(&format!("<f>{}", deep(20)), vocab::RDF_XML_LITERAL), Some(false));
    }

    #[test]
    fn value_spaces_as_the_reference_defines_them() {
        let mut d = Dictionary::new();
        let mut typed = |lexical: &str, datatype: TermId| d.intern_typed_literal(lexical, datatype);
        let five_int = typed("5", vocab::XSD_INT);
        let five_integer = typed("5", vocab::XSD_INTEGER);
        let big = typed("2147483648", vocab::XSD_INTEGER);
        let abc = typed("abc", vocab::XSD_INTEGER);
        let x = typed("x", vocab::XSD_STRING);
        let xml = typed("<a/>", vocab::RDF_XML_LITERAL);
        let decimal = typed("5", vocab::XSD_DECIMAL);
        let tagged = d.intern_lang_literal("x", "en");
        let iri = d.intern_iri("http://example.org/x");
        let space = |id, datatype| in_value_space(&d, id, datatype);
        assert_eq!(space(five_int, vocab::XSD_INTEGER), Some(true));
        assert_eq!(space(five_int, vocab::XSD_INT), Some(true));
        assert_eq!(space(five_int, vocab::XSD_STRING), Some(false));
        assert_eq!(space(five_integer, vocab::XSD_INT), Some(true));
        assert_eq!(space(big, vocab::XSD_INT), Some(false));
        assert_eq!(space(big, vocab::XSD_INTEGER), Some(true));
        assert_eq!(space(abc, vocab::XSD_INTEGER), Some(false));
        assert_eq!(space(abc, vocab::XSD_STRING), Some(false));
        assert_eq!(space(x, vocab::XSD_STRING), Some(true));
        assert_eq!(space(x, vocab::XSD_INTEGER), Some(false));
        assert_eq!(space(x, vocab::RDF_LANG_STRING), Some(false));
        assert_eq!(space(tagged, vocab::XSD_STRING), Some(false));
        assert_eq!(space(tagged, vocab::RDF_LANG_STRING), Some(true));
        assert_eq!(space(xml, vocab::RDF_XML_LITERAL), Some(true));
        assert_eq!(space(xml, vocab::XSD_STRING), Some(false));
        // Not a literal of a checked datatype: unknown.
        assert_eq!(space(decimal, vocab::XSD_INTEGER), None);
        assert_eq!(space(iri, vocab::XSD_INTEGER), None);
    }

    #[test]
    fn a_family_reports_through_the_context() {
        // The findings are raw: duplicates are the reasoner's to remove.
        let run = run("ex:x ex:p \"a\"^^xsd:int . ex:y ex:p \"a\"^^xsd:int .");
        assert_eq!(run.findings.len(), 2);
        assert!(run.findings.iter().all(|f: &Finding| f.code == "dt-not-type"));
    }

    // -----------------------------------------------------------------------
    // Scaling
    // -----------------------------------------------------------------------

    /// A fixed schema (disjoint and complementary classes, disjoint,
    /// asymmetric and irreflexive properties) over `n` individuals, about
    /// 13 triples each, literals included, with a known number of findings:
    /// the dictionary and the store, its triples queued.
    fn instances(n: usize) -> (Dictionary, Store, usize) {
        let mut d = Dictionary::new();
        let mut store = Store::new();
        let ex = |d: &mut Dictionary, local: &str| d.intern_iri(&format!("http://example.org/{local}")).0;
        let [a, b, c, comp, p, q, asym, irr, age, label] =
            ["A", "B", "C", "D", "p", "q", "asym", "irr", "age", "label"].map(|l| ex(&mut d, l));
        let ty = vocab::RDF_TYPE.0;
        store.insert(a, vocab::OWL_DISJOINT_WITH.0, b);
        store.insert(c, vocab::OWL_COMPLEMENT_OF.0, comp);
        store.insert(p, vocab::OWL_PROPERTY_DISJOINT_WITH.0, q);
        store.insert(asym, ty, vocab::OWL_ASYMMETRIC_PROPERTY.0);
        store.insert(irr, ty, vocab::OWL_IRREFLEXIVE_PROPERTY.0);
        let x: Vec<u32> = (0..=n).map(|i| ex(&mut d, &format!("x{i}"))).collect();
        for i in 0..n {
            let (xi, next) = (x[i], x[i + 1]);
            let every = |k: usize| i % k == 0;
            store.insert(xi, ty, a);
            if every(8) {
                store.insert(xi, ty, b);
            }
            if every(2) {
                store.insert(xi, ty, c);
            }
            if every(16) {
                store.insert(xi, ty, comp);
            }
            if every(32) {
                store.insert(xi, ty, vocab::OWL_NOTHING.0);
            }
            store.insert(xi, p, next);
            if every(4) {
                store.insert(xi, q, next);
            }
            store.insert(xi, asym, next);
            if every(8) {
                store.insert(next, asym, xi);
            }
            store.insert(xi, irr, next);
            if every(16) {
                store.insert(xi, irr, xi);
            }
            let years = d.intern_typed_literal(&i.to_string(), vocab::XSD_INT).0;
            store.insert(xi, age, years);
            store.insert(years, ty, vocab::XSD_INTEGER.0);
            let name = d.intern_typed_literal(&format!("x {i}"), vocab::XSD_STRING).0;
            store.insert(xi, label, name);
            if every(8) {
                let bad = d.intern_typed_literal(&format!("{i}a"), vocab::XSD_INTEGER).0;
                store.insert(xi, age, bad);
            }
            if every(16) {
                store.insert(name, ty, vocab::XSD_INTEGER.0);
            }
        }
        // cax-dw, cls-com, cls-nothing2, prp-pdw, prp-asyp (both ways),
        // prp-irp, dt-not-type (ill-typed, then typed).
        let expected = n / 8 + n / 16 + n / 32 + n / 4 + 2 * (n / 8) + n / 16 + n / 8 + n / 16;
        (d, store, expected)
    }

    /// The family's rounds over `input`: the seconds spent in the family
    /// (the engine's own steps not counted), and the distinct findings.
    fn timed_run((d, mut store, _): (Dictionary, Store, usize)) -> (f64, usize) {
        let mut findings = Vec::new();
        let mut spent = std::time::Duration::ZERO;
        let mut cx = Context {
            terms: &d,
            findings: &mut findings,
        };
        store.run(|round| {
            let start = std::time::Instant::now();
            rules(round, &mut cx);
            spent += start.elapsed();
        });
        findings.sort_unstable();
        findings.dedup();
        (spent.as_secs_f64(), findings.len())
    }

    /// With a fixed schema, the family's run is linear in the instance
    /// data. Only the family's calls are timed: the engine's sorting of
    /// the input into tables is the engine's own scaling test.
    #[test]
    fn scaling_datatype_rules_linear_in_instances() {
        let _lock = crate::scaling::lock();
        const LEAST: usize = 1_024;
        let sizes = [LEAST, 2 * LEAST, 4 * LEAST, 8 * LEAST, 16 * LEAST];
        for n in sizes {
            let input = instances(n);
            let expected = input.2;
            assert_eq!(timed_run(input).1, expected, "findings at {n}");
        }
        // Each measurement runs the same number of individuals in all
        // (16 inputs of the least size, one of the largest), so short runs
        // are not timed against the clock alone.
        crate::scaling::check(
            "scaling_datatype_rules_linear_in_instances",
            crate::scaling::Growth::Linear,
            &sizes,
            |n| {
                let reps = 16 * LEAST / n;
                let inputs: Vec<_> = (0..reps).map(|_| instances(n)).collect();
                inputs.into_iter().map(|i| timed_run(i).0).sum::<f64>() / reps as f64
            },
        );
    }
}
