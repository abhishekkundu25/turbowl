//! Diagnostics: findings about the data (ticket 09). The rules push
//! internal findings (term ids); [`Diagnostic::from_finding`] makes the
//! public form, with `oxrdf` terms and a message.

use crate::rules::Finding;
use crate::term::{Dictionary, TermId, vocab};
use oxrdf::Term;

/// How serious a diagnostic is. Every inconsistency of the rule set is an
/// [`Severity::Error`]; [`Severity::Warning`] is kept for later checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    Error,
    Warning,
}

/// A finding about the data: an inconsistency (a W3C OWL 2 RL rule that
/// concludes `false`) or an ill-typed literal.
///
/// `code` is the rule's W3C name, and `terms` holds the terms involved in
/// one order per code:
///
/// | Code | Terms |
/// |---|---|
/// | `cax-dw` | individual, class 1, class 2 (as stated in `class1 owl:disjointWith class2`) |
/// | `prp-pdw` | property 1, property 2 (as stated in `owl:propertyDisjointWith`), subject, object |
/// | `cls-nothing2` | individual |
/// | `prp-asyp` | property, subject, object (`subject property object` and its reverse both hold) |
/// | `prp-irp` | property, individual |
/// | `cls-com` | class 1, class 2 (as stated in `class1 owl:complementOf class2`), individual |
/// | `dt-not-type` | literal, datatype (the literal's own when it is ill-typed, or a checked datatype the closure types it with, as by `rdfs:range`) |
///
/// dt-not-type checks `xsd:string`, `xsd:integer`, `xsd:int`,
/// `rdf:langString` and `rdf:XMLLiteral`; a literal of another datatype is
/// never reported.
///
/// The rules run over generalized triples, so a term may be a literal
/// where RDF allows only an IRI or a blank node (an individual that a
/// range typed, for instance). Blank nodes are labelled as on output. Each
/// diagnostic is reported once.
///
/// The message is one sentence naming the terms: IRIs in the `rdf:`,
/// `rdfs:`, `owl:` and `xsd:` namespaces are written with those prefixes,
/// other IRIs as `<...>`; blank nodes as `_:label`; literals as in
/// N-Triples, their datatype shortened the same way (and left out for
/// `xsd:string`). The wording may change; read `code` and `terms`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Diagnostic {
    /// The W3C rule that found it: `cax-dw`, `prp-irp`, `dt-not-type`, ...
    pub code: &'static str,
    pub severity: Severity,
    /// A sentence naming the terms.
    pub message: String,
    /// The terms involved, in the order the table above gives for `code`.
    pub terms: Vec<Term>,
}

impl Diagnostic {
    /// The public form of a rule's finding: its terms read from the
    /// dictionary, its severity and its message.
    #[allow(dead_code)] // the reasoner (ticket 10) calls it
    pub(crate) fn from_finding(finding: &Finding, terms: &Dictionary) -> Self {
        let terms: Vec<Term> = finding.terms.iter().map(|&t| terms.to_oxrdf(TermId(t))).collect();
        Self {
            code: finding.code,
            severity: Severity::Error,
            message: message(finding.code, &terms),
            terms,
        }
    }
}

/// The sentence for a finding.
fn message(code: &str, terms: &[Term]) -> String {
    let names: Vec<String> = terms.iter().map(name).collect();
    match (code, names.as_slice()) {
        ("cax-dw", [x, c1, c2]) => {
            format!("{x} is an instance of {c1} and of {c2}, which are disjoint classes.")
        }
        ("prp-pdw", [p1, p2, x, y]) => {
            format!("{x} is linked to {y} by both {p1} and {p2}, which are disjoint properties.")
        }
        ("cls-nothing2", [x]) => format!("{x} is an instance of owl:Nothing, which has no instances."),
        ("prp-asyp", [p, x, y]) => {
            format!("{x} is linked to {y} and {y} to {x} by {p}, which is asymmetric.")
        }
        ("prp-irp", [p, x]) => format!("{x} is linked to itself by {p}, which is irreflexive."),
        ("cls-com", [c1, c2, x]) => {
            format!("{x} is an instance of {c1} and of its complement {c2}.")
        }
        ("dt-not-type", [literal, datatype]) => {
            format!("{literal} is not a value of the datatype {datatype}.")
        }
        _ => format!("{code}: {}.", names.join(", ")),
    }
}

/// A term as a message writes it (see [`Diagnostic`]).
fn name(term: &Term) -> String {
    match term {
        Term::NamedNode(iri) => short(iri.as_str()),
        Term::Literal(literal) => {
            // A simple literal's display is the quoted, escaped lexical form.
            let mut out = oxrdf::LiteralRef::new_simple_literal(literal.value()).to_string();
            if let Some(language) = literal.language() {
                out.push('@');
                out.push_str(language);
            } else if literal.datatype().as_str() != vocab::XSD_STRING_IRI {
                out.push_str("^^");
                out.push_str(&short(literal.datatype().as_str()));
            }
            out
        }
        _ => term.to_string(),
    }
}

/// An IRI with a known prefix, or `<iri>`.
fn short(iri: &str) -> String {
    for (prefix, namespace) in vocab::PREFIXES {
        if let Some(local) = iri.strip_prefix(namespace)
            && !local.is_empty()
            && local.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return format!("{prefix}:{local}");
        }
    }
    format!("<{iri}>")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::term::DocId;
    use oxrdf::{BlankNode, Literal, NamedNode};

    const EX: &str = "http://example.org/";

    fn finding(code: &'static str, terms: &[TermId]) -> Finding {
        Finding {
            code,
            terms: terms.iter().map(|t| t.0).collect(),
        }
    }

    #[test]
    fn a_finding_becomes_an_error_with_its_terms_in_order() {
        let mut d = Dictionary::new();
        let [x, a, b] = ["x", "A", "B"].map(|l| d.intern_iri(&format!("{EX}{l}")));
        let diagnostic = Diagnostic::from_finding(&finding("cax-dw", &[x, a, b]), &d);
        assert_eq!(diagnostic.code, "cax-dw");
        assert_eq!(diagnostic.severity, Severity::Error);
        assert_eq!(
            diagnostic.terms,
            ["x", "A", "B"].map(|l| Term::from(NamedNode::new_unchecked(format!("{EX}{l}"))))
        );
        assert_eq!(
            diagnostic.message,
            "<http://example.org/x> is an instance of <http://example.org/A> and of \
             <http://example.org/B>, which are disjoint classes."
        );
    }

    #[test]
    fn every_code_has_a_sentence_naming_its_terms() {
        let mut d = Dictionary::new();
        let [p, q, x, y, a, b] = ["p", "q", "x", "y", "A", "B"].map(|l| d.intern_iri(&format!("{EX}{l}")));
        let blank = d.intern_blank(DocId(3), "b1");
        let bad = d.intern_typed_literal("abc", vocab::XSD_INTEGER);
        let plain = d.intern_typed_literal("say \"hi\"", vocab::XSD_STRING);
        let tagged = d.intern_lang_literal("chat", "fr");
        let dt = d.intern_iri(&format!("{EX}dt"));
        let custom = d.intern_typed_literal("1", dt);
        let cases: [(&'static str, Vec<TermId>, &str); 9] = [
            (
                "prp-pdw",
                vec![p, q, x, y],
                "<http://example.org/x> is linked to <http://example.org/y> by both \
                 <http://example.org/p> and <http://example.org/q>, which are disjoint properties.",
            ),
            (
                "cls-nothing2",
                vec![blank],
                "_:b1_3 is an instance of owl:Nothing, which has no instances.",
            ),
            (
                "prp-asyp",
                vec![p, x, bad],
                "<http://example.org/x> is linked to \"abc\"^^xsd:integer and \"abc\"^^xsd:integer \
                 to <http://example.org/x> by <http://example.org/p>, which is asymmetric.",
            ),
            (
                "prp-irp",
                vec![p, tagged],
                "\"chat\"@fr is linked to itself by <http://example.org/p>, which is irreflexive.",
            ),
            (
                "cls-com",
                vec![a, b, x],
                "<http://example.org/x> is an instance of <http://example.org/A> and of its \
                 complement <http://example.org/B>.",
            ),
            (
                "dt-not-type",
                vec![bad, vocab::XSD_INTEGER],
                "\"abc\"^^xsd:integer is not a value of the datatype xsd:integer.",
            ),
            (
                "dt-not-type",
                vec![plain, vocab::XSD_INT],
                "\"say \\\"hi\\\"\" is not a value of the datatype xsd:int.",
            ),
            (
                "dt-not-type",
                vec![custom, vocab::RDF_XML_LITERAL],
                "\"1\"^^<http://example.org/dt> is not a value of the datatype rdf:XMLLiteral.",
            ),
            (
                "cax-dw",
                vec![bad, vocab::OWL_THING, vocab::RDFS_LITERAL],
                "\"abc\"^^xsd:integer is an instance of owl:Thing and of rdfs:Literal, which are \
                 disjoint classes.",
            ),
        ];
        for (code, ids, message) in cases {
            let diagnostic = Diagnostic::from_finding(&finding(code, &ids), &d);
            assert_eq!(diagnostic.message, message, "{code}");
            assert_eq!(diagnostic.severity, Severity::Error);
            let terms: Vec<Term> = ids.iter().map(|&t| d.to_oxrdf(t)).collect();
            assert_eq!(diagnostic.terms, terms, "{code}");
        }
        // Generalized terms come out as they are.
        let diagnostic = Diagnostic::from_finding(&finding("cls-nothing2", &[blank]), &d);
        assert_eq!(diagnostic.terms, [Term::from(BlankNode::new_unchecked("b1_3"))]);
        let diagnostic = Diagnostic::from_finding(&finding("dt-not-type", &[bad, vocab::XSD_INTEGER]), &d);
        assert_eq!(
            diagnostic.terms,
            [
                Term::from(Literal::new_typed_literal("abc", oxrdf::vocab::xsd::INTEGER)),
                Term::from(oxrdf::vocab::xsd::INTEGER.into_owned()),
            ]
        );
    }

    #[test]
    fn an_unknown_code_still_names_its_terms() {
        let mut d = Dictionary::new();
        let x = d.intern_iri(&format!("{EX}x"));
        let diagnostic = Diagnostic::from_finding(&finding("other", &[x, vocab::OWL_CLASS]), &d);
        assert_eq!(diagnostic.message, "other: <http://example.org/x>, owl:Class.");
    }

    #[test]
    fn only_simple_local_names_are_shortened() {
        assert_eq!(short("http://www.w3.org/2002/07/owl#sameAs"), "owl:sameAs");
        assert_eq!(
            short("http://www.w3.org/2002/07/owl#"),
            "<http://www.w3.org/2002/07/owl#>"
        );
        assert_eq!(
            short("http://www.w3.org/2001/XMLSchema#a/b"),
            "<http://www.w3.org/2001/XMLSchema#a/b>"
        );
        assert_eq!(short("http://example.org/x"), "<http://example.org/x>");
    }
}
