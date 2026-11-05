//! The datatype family (switch `checks`): dt-not-type for the checked
//! datatypes. Ticket 09 replaces the five checked datatypes with the value
//! spaces of every OWL 2 RL datatype, which dt-type2, dt-eq and dt-diff
//! (switches `trivial` and `literal-values`) read too.
//!
//! The rule derives no triple: it pushes a finding through
//! [`Context::report`], the literal then the datatype, as the public
//! `Diagnostic` documents.
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
//!   That index is built lazily ([`Round::by_object`]): the round after the
//!   first request, it holds every typing as recent, so nothing is lost.
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

use super::Context;
use crate::engine::{Pair, Round, Variable, gallop};
use crate::term::{Dictionary, Kind, Term, TermId, vocab};

/// The datatype rules have no empty-body rule.
pub(crate) const FACTS: &[[TermId; 3]] = &[];

/// The datatypes dt-not-type checks.
const CHECKED: [TermId; 5] = [
    vocab::XSD_STRING,
    vocab::XSD_INTEGER,
    vocab::XSD_INT,
    vocab::RDF_LANG_STRING,
    vocab::RDF_XML_LITERAL,
];

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

/// dt-not-type, the typed half: a recent `x rdf:type dt` for a checked
/// `dt`, and each literal member of `x`'s class whose value `dt`'s value
/// space does not hold.
fn typed_literals(round: &Round<'_>, types: &Variable<Pair>, cx: &mut Context<'_>) {
    let canon = round.canon();
    for dt in CHECKED {
        for &(_, x) in with_key(types.recent(), round.constant(dt.0)) {
            for m in canon.members(x).iter() {
                if in_value_space(cx.terms, TermId(m), dt) == Some(false) {
                    cx.report("dt-not-type", &[m, dt.0]);
                }
            }
        }
    }
}

/// dt-not-type, the typed half, for the literals that joined a class in the
/// last step: against every checked datatype their new representative is
/// typed with (read by subject, so no object index is needed).
fn moved_literals(round: &Round<'_>, ty: u32, cx: &mut Context<'_>) {
    let moved = round.moved();
    if moved.is_empty() {
        return;
    }
    let Some(types) = round.by_subject(ty) else { return };
    let types = types.all();
    let checked = CHECKED.map(|dt| (round.constant(dt.0), dt));
    for &m in moved {
        if cx.terms.kind(TermId(m)) != Kind::Literal {
            continue;
        }
        let r = round.canon().find(m);
        types.for_each_value(r, |class| {
            for &(c, dt) in &checked {
                if c == class && in_value_space(cx.terms, TermId(m), dt) == Some(false) {
                    cx.report("dt-not-type", &[m, dt.0]);
                }
            }
        });
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
}
