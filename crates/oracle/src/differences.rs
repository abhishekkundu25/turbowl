//! The oracle faults (`CONTEXT.md`): places where an oracle is wrong and the
//! W3C OWL 2 RL rules are not. Each has an id, a test below showing that the
//! other oracles agree with the W3C rule, and a filter that removes exactly
//! the affected triples or findings before a closure is compared with the
//! faulty oracle's. There are no other filters (ADR 0004).
//!
//! All fifteen are faults of OWL-RL (owlrl 7.6.2 on rdflib 7.6.0). Nemo,
//! HermiT and the W3C tests have none so far. Where OWL-RL only derives or
//! reports more than the W3C rules (F8, F11, F13, F14, and parts of F9 and
//! F10), containment never sees it: the fault is listed with its test and
//! filters nothing.

use crate::{Closure, Finding, Input};
use oxrdf::vocab::{rdf, rdfs, xsd};
use oxrdf::{NamedNodeRef, NamedOrBlankNode, Term};
use std::collections::{HashMap, HashSet};
use turbowl::{Switch, Switches};

/// A known fault of an oracle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Fault {
    /// OWL-RL's scm-sco and scm-spo never derive `c rdfs:subClassOf c` or
    /// `p rdfs:subPropertyOf p`, so neither scm-eqc2 nor scm-eqp2 gives
    /// `c owl:equivalentClass c` or `p owl:equivalentProperty p` from them
    /// (nor, by eq-rep-p, `c q c` for a `q` made `owl:sameAs` one of the
    /// four).
    F1,
    /// OWL-RL's cls-com names the second class where the individual belongs.
    F2,
    /// OWL-RL's dt-not-type accepts a lexical form with leading or trailing whitespace.
    F3,
    /// OWL-RL fails on a cyclic list.
    F4,
    /// OWL-RL's dt-not-type checks a literal only against its own datatype.
    F5,
    /// OWL-RL's eq-diff2, eq-diff3 and prp-adp miss positions of a list.
    F6,
    /// OWL-RL's dt-not-type reads numbers with Python's `int()` and `Decimal()`.
    F7,
    /// OWL-RL's dt-not-type rejects zeros, INF, NaN and rounded numerals of
    /// `xsd:double` and `xsd:float`.
    F8,
    /// OWL-RL's dt-not-type has the wrong lexical spaces for booleans,
    /// binaries, the XML name types, `xsd:normalizedString` and `xsd:anyURI`.
    F9,
    /// OWL-RL's dt-not-type reads `xsd:dateTime` with `time.strptime`.
    F10,
    /// OWL-RL's dt-not-type parses an `rdf:XMLLiteral` as a whole document.
    F11,
    /// rdflib rewrites the lexical forms of `xsd:token` and
    /// `xsd:normalizedString` literals.
    F12,
    /// OWL-RL's prp-key never compares a key's last property.
    F13,
    /// OWL-RL's cardinality rules compare Python values, so a double, a float
    /// or a boolean cardinality fires them.
    F14,
    /// OWL-RL skips dt-eq and dt-diff, and its dt-type2 types a literal with
    /// its own datatype and that datatype's supertypes only.
    F15,
}

impl Fault {
    pub const ALL: [Self; 15] = [
        Self::F1,
        Self::F2,
        Self::F3,
        Self::F4,
        Self::F5,
        Self::F6,
        Self::F7,
        Self::F8,
        Self::F9,
        Self::F10,
        Self::F11,
        Self::F12,
        Self::F13,
        Self::F14,
        Self::F15,
    ];

    /// The oracle at fault, as the triangulation table names it.
    pub fn oracle(self) -> &'static str {
        "OWL-RL"
    }

    /// What the oracle does wrong, and what the W3C rule says instead.
    pub fn description(self) -> &'static str {
        match self {
            Self::F1 => {
                "owlrl 7.6.2 skips c rdfs:subClassOf c in scm-sco (OWLRL.py:883-888, `if c1 != c2` and \
                 `if c1 != c3`) and p rdfs:subPropertyOf p in scm-spo (OWLRL.py:908-916, `s != o`, \
                 `if p1 != p2` and `if p1 != p3`), and derives them only through scm-cls and scm-op, which \
                 need `c a owl:Class` or a property type; so neither scm-eqc2 nor scm-eqp2 (OWLRL.py:889, \
                 919) gives `c owl:equivalentClass c` or `p owl:equivalentProperty p` from them. The W3C \
                 rules have no such condition: `A subClassOf B . B subClassOf A` gives A subClassOf A and \
                 A equivalentClass A, `p equivalentProperty q` gives p subPropertyOf p and p \
                 equivalentProperty p, and eq-rep-p copies each to every property made owl:sameAs the \
                 predicate"
            }
            Self::F2 => {
                "owlrl 7.6.2 formats cls-com's message with `x`, which is the second class there \
                 (OWLRL.py:684-691), so its finding is (c1, c2, c2), not (c1, c2, individual)"
            }
            Self::F3 => {
                "owlrl 7.6.2 accepts \" 42\"^^xsd:int (Python's int() strips the spaces); RDF does not \
                 collapse whitespace, and the W3C test rdf-mt xmlsch-02-whitespace-facet-2 expects \
                 \" 3 \"^^xsd:int to be ill-typed"
            }
            Self::F4 => {
                "rdflib's Collection raises \"List contains a recursive rdf:rest reference\" on a cyclic \
                 list, so owlrl 7.6.2 stops with an error; the W3C rules derive nothing from a list that \
                 never reaches rdf:nil"
            }
            Self::F5 => {
                "owlrl 7.6.2 runs dt-not-type only on literals against their own datatype \
                 (OWLRL.py:217-232), never on `lt rdf:type dt` triples, so a literal typed by prp-rng is \
                 not checked; the W3C dt-not-type matches any such triple, and the W3C test rdf-mt \
                 datatypes-range-clash expects \"25\"^^xsd:integer in an xsd:string range to be \
                 inconsistent"
            }
            Self::F6 => {
                "owlrl 7.6.2 reads the list of eq-diff2, eq-diff3 (OWLRL.py:416-436) and prp-adp \
                 (OWLRL.py:517-531) through rdflib's `items`, one rdf:first per cell, pairs positions \
                 with `for j in range(i + 1, len(list) - 1)`, which never reaches the last member, and \
                 for eq-diff2 and eq-diff3 skips zi = zj; the W3C rules pair every two positions \
                 i < j of the list, with every rdf:first of each cell, so `(a b c)` with \
                 `a owl:sameAs b` gives (a, a), (a, b), (b, a) and (b, b), and `(p q)` gives prp-adp"
            }
            Self::F7 => {
                "owlrl 7.6.2 checks a literal of xsd:decimal, xsd:integer and its subtypes, xsd:double or \
                 xsd:float by reading it with Python (OWLRL.py:218-232; DatatypeHandling.py:118-132 and \
                 267-311 `Decimal(v)`, 247-262 and 669 `int`), which takes digits of any script, underscores \
                 between digits and, in Decimal, NaN and Infinity: it accepts \"\u{663}\"^^xsd:integer, \
                 \"1_000\"^^xsd:decimal and \"NaN\"^^xsd:decimal. XML Schema 1.1 (3.3.3 to 3.3.5, 3.4.13) \
                 writes these lexical spaces in ASCII digits, decimal's as \
                 (\\+|-)?([0-9]+(\\.[0-9]*)?|\\.[0-9]+) with no NaN, so the W3C dt-not-type reports each"
            }
            Self::F8 => {
                "owlrl 7.6.2 rejects an xsd:double whose Decimal(v) is not strictly between 1.0E-330 and \
                 1.0E+310 in magnitude, an xsd:float outside 1.0E-50 and 1.0E+40 (DatatypeHandling.py:267-311), \
                 so \"0\", \"-0\", \"INF\", \"NaN\", \"1E400\" and \"1e-400\" (and \"1E40\"^^xsd:float) are \
                 reported ill-typed. XML Schema 1.1 (3.3.4, 3.3.5) lists INF, +INF, -INF and NaN as lexical \
                 forms, maps \"-0\" to negative zero and rounds every other numeral (floatingPointRound, to \
                 zero or INF out of range): each has a value, and the W3C dt-not-type reports none"
            }
            Self::F9 => {
                "owlrl 7.6.2's readers (DatatypeHandling.py) depart from XML Schema 1.1: _strToBool \
                 (102-113) lowercases, so \"TRUE\" passes (3.3.2: 'true' | 'false' | '1' | '0'); \
                 _strToHexBinary (316-326) never rejects an odd length (`(length / 2) * 2` is a float) and \
                 reads int(v, 16), so \"0FB\" and \"0x0F\" pass and \"\" fails (3.3.15: hexOctet*); \
                 _strToBase64Binary (179-194) takes nonzero padding bits (\"QR==\") and rejects spaces and \
                 \"\" (3.3.16: B04char [AQgw], B16char [AEIMQUYcgkosw048], B64 ::= B64char #x20?, the whole \
                 optional); normalizedString's `[^\\n\\t\\r]+` (672) rejects \"\" (3.4.1); Name, NCName and \
                 NMTOKEN match \\w (546-578), which takes ª ² ³ µ ¹ º ¼ ½ ¾, no XML NameChar, and refuses · \
                 and combining marks, NameChars in every XML edition (3.4.4, 3.4.6, 3.4.7); _strToAnyURI \
                 (141-174) rejects \"?q\" and \":x\", where 3.3.17's lexical space is every string of XML \
                 characters. The W3C dt-not-type reports the forms OWL-RL accepts and none it rejects"
            }
            Self::F10 => {
                "owlrl 7.6.2 reads xsd:dateTime and xsd:dateTimeStamp with time.strptime(v, \
                 \"%Y-%m-%dT%H:%M:%S\") after cutting off any `[+-]dd:dd` time zone and a fraction read by \
                 int() as microseconds (DatatypeHandling.py:75-97, 331-386). It accepts \"+15:00\", \
                 \"+01:60\", a one-digit month, day or time field and a lowercase \"t\", which XML Schema \
                 1.1's dateTimeLexicalRep (3.3.7.2; D.2.2: monthFrag ('0' [1-9]) | ('1' [0-2]), \
                 timezoneFrag ('0' digit | '1' [0-3]) ':' minuteFrag | '14:00', the literal 'T') rejects; \
                 and it rejects year 0000, negative and five-digit years, 24:00:00 and seven fraction \
                 digits, which yearFrag, endOfDayFrag and secondFrag ('.' digit+) allow (5.4: a form \
                 beyond a processor's capacity 'must not be treated as invalid'). The W3C dt-not-type \
                 reports the forms OWL-RL accepts and none it rejects"
            }
            Self::F11 => {
                "owlrl 7.6.2 checks an rdf:XMLLiteral with xml.dom.minidom.parseString \
                 (DatatypeHandling.py:524-536), which needs one root element, so \"plain text\" and \"\" are \
                 reported ill-typed; RDF Concepts 5.1 (2004 and 1.1) makes the lexical space well-balanced, \
                 self-contained XML content, which text and the empty string are, so the W3C dt-not-type \
                 reports neither"
            }
            Self::F12 => {
                "rdflib 7.6.0 replaces tabs, line feeds and carriage returns by spaces in every \
                 xsd:normalizedString and xsd:token literal, and strips and collapses the spaces of an \
                 xsd:token, as it builds the literal, whatever NORMALIZE_LITERALS says (rdflib/term.py:711-715, \
                 2284-2299). OWL-RL's graph holds \"a b\"^^xsd:token for the input's \"a  b\"^^xsd:token, \
                 another RDF term (RDF 1.1 Concepts 3.3: literals are term-equal only when their lexical \
                 forms are), and checks the rewritten form, so it misses dt-not-type on \"a  b\"^^xsd:token \
                 and \"a\\tb\"^^xsd:normalizedString, which XML Schema 1.1 (3.4.1, 3.4.2) rejects. The W3C \
                 rules keep the input's literal"
            }
            Self::F13 => {
                "owlrl 7.6.2's prp-key compares the key values of y with x's only for \
                 `range(0, len(pis) - 1)` (OWLRL.py:578-619, at 613), never the last property, so a \
                 one-property key makes x owl:sameAs every other individual of the class; the W3C prp-key \
                 needs T(?y, ?pi, ?zi) for every key property"
            }
            Self::F14 => {
                "owlrl 7.6.2's cls-maxc1, cls-maxc2 and cls-maxqc1 to cls-maxqc4 test the cardinality's \
                 Python value, `x.value == 0` and `x.value == 1` (OWLRL.py:729-790), and Python makes 1.0 and \
                 True equal to 1, 0.0 and False to 0, so \"1\"^^xsd:double and \"false\"^^xsd:boolean fire \
                 them. The W3C rules name \"0\"^^xsd:nonNegativeInteger and \"1\"^^xsd:nonNegativeInteger, \
                 which dt-eq and eq-rep-o reach from a literal of the same data value (\"1\"^^xsd:integer \
                 and \"1.0\"^^xsd:decimal fire them), but the value spaces of xsd:double, xsd:float and \
                 xsd:decimal are pairwise disjoint (OWL 2 Structural Specification 4.2) and xsd:boolean's \
                 holds no numbers (XML Schema 1.1 3.3.2)"
            }
            Self::F15 => {
                "owlrl 7.6.2 skips dt-eq and dt-diff (OWLRL.py:234-237: \"This rules are skipped on purpose \
                 at the moment\"), and its dt-type2 types a literal of a datatype of its list with that \
                 datatype (OWLRL.py:215-220) and the supertypes its table gives (OWLRL.py:191-198, \
                 XsdDatatypes.py:85: xsd:int's are xsd:long, xsd:integer and xsd:decimal), a term made \
                 owl:sameAs the literal too (OWLRL.py:248-256). OWL 2 Profiles, Table 8: dt-eq makes every \
                 two literals with the same value owl:sameAs (\"1\"^^xsd:integer and \"1.0\"^^xsd:decimal), \
                 dt-diff makes those with different values owl:differentFrom (so two made owl:sameAs are \
                 inconsistent, eq-diff1), and dt-type2 types a literal with every datatype whose value \
                 space holds its value (\"5\"^^xsd:int is an xsd:byte and an xsd:unsignedByte)"
            }
        }
    }

    /// What the comparison with the faulty oracle does about it.
    pub fn handling(self) -> &'static str {
        match self {
            Self::F1 => {
                "drops `c p c` triples OWL-RL does not have from the other closure, for p rdfs:subClassOf, \
                 owl:equivalentClass, rdfs:subPropertyOf, owl:equivalentProperty and their aliases"
            }
            Self::F2 => "compares cls-com findings on the two classes only, on both sides",
            Self::F3 => {
                "drops dt-not-type findings OWL-RL does not have whose literal has leading or trailing \
                 whitespace from the other closure"
            }
            Self::F4 => "OWL-RL's failure on an input with a cyclic list is known; nothing is compared with it",
            Self::F5 => {
                "drops dt-not-type findings OWL-RL does not have whose datatype is not the literal's own \
                 from the other closure"
            }
            Self::F6 => "drops eq-diff2, eq-diff3 and prp-adp findings OWL-RL does not have from the other closure",
            Self::F7 => {
                "drops dt-not-type findings OWL-RL does not have, of a number against its own datatype whose \
                 lexical form holds an underscore or a character outside ASCII, or is a decimal NaN or \
                 Infinity, from the other closure"
            }
            Self::F9 => {
                "drops dt-not-type findings OWL-RL does not have, against the literal's own datatype, of a \
                 boolean in another case, a hexBinary int(v, 16) reads, a base64Binary with nonzero padding \
                 bits, or a name type holding one of ª ² ³ µ ¹ º ¼ ½ ¾, from the other closure; the forms \
                 OWL-RL rejects need nothing"
            }
            Self::F10 => {
                "drops dt-not-type findings OWL-RL does not have, against the literal's own datatype, of a \
                 dateTime strptime reads with a lowercase t, a one-digit field or a time zone past +14:00 or \
                 past 59 minutes, from the other closure; the forms OWL-RL rejects need nothing"
            }
            Self::F12 => {
                "drops the triples and findings OWL-RL does not have that hold an xsd:token or \
                 xsd:normalizedString literal rdflib rewrites, from the other closure"
            }
            Self::F8 | Self::F11 | Self::F13 | Self::F14 => {
                "none: OWL-RL derives or reports more than the W3C rules, which containment never sees"
            }
            Self::F15 => {
                "drops from the other closure the triples and findings OWL-RL does not have that the \
                 complete configuration without literal-values does not give either (dt-eq, dt-diff and \
                 what follows from them, as TurbOWL computes it: `ours = Nemo` holds it to the rules), \
                 and the typings `x rdf:type dt` OWL-RL does not have of a term `x` made owl:sameAs a \
                 literal whose value dt's value space holds (dt-type2)"
            }
        }
    }

    /// True when the fault shows between a closure (`ours`, the one judged)
    /// and the faulty oracle's (`theirs`) on this input.
    pub fn shows(self, input: &Input, ours: &Closure, theirs: &Closure) -> bool {
        match self {
            Self::F1 => {
                let aliases = schema_aliases(ours);
                ours.triples
                    .iter()
                    .any(|t| reflexive(t, &aliases) && !theirs.triples.contains(t))
            }
            Self::F2 => [ours, theirs]
                .iter()
                .any(|c| c.findings.iter().any(|f| f.code == "cls-com")),
            Self::F3 => ours
                .findings
                .iter()
                .any(|f| padded_literal(f) && !theirs.findings.contains(f)),
            Self::F4 => has_cyclic_list(input),
            Self::F5 => ours
                .findings
                .iter()
                .any(|f| typed_elsewhere(f) && !theirs.findings.contains(f)),
            Self::F6 => ours
                .findings
                .iter()
                .any(|f| by_list_position(f) && !theirs.findings.contains(f)),
            Self::F7 => missed(ours, theirs, python_number),
            Self::F8 => over_reported(ours, theirs, &[xsd::DOUBLE, xsd::FLOAT]),
            Self::F9 => missed(ours, theirs, python_lexical) || over_reported(ours, theirs, F9_DATATYPES),
            Self::F10 => {
                missed(ours, theirs, strptime_date_time)
                    || over_reported(ours, theirs, &[xsd::DATE_TIME, xsd::DATE_TIME_STAMP])
            }
            Self::F11 => over_reported(ours, theirs, &[rdf::XML_LITERAL]),
            Self::F12 => {
                ours.triples
                    .iter()
                    .any(|t| rewritten_object(t) && !theirs.triples.contains(t))
                    || missed(ours, theirs, holds_rewritten)
            }
            Self::F13 => {
                has_predicate(input, OWL_HAS_KEY)
                    && theirs.triples.iter().any(|t| {
                        t.predicate.as_str() == OWL_SAME_AS
                            && Term::from(t.subject.clone()) != t.object
                            && !ours.triples.contains(t)
                    })
            }
            Self::F14 => input.triples().iter().any(|t| {
                [OWL_MAX_CARDINALITY, OWL_MAX_QUALIFIED_CARDINALITY].contains(&t.predicate.as_str())
                    && matches!(&t.object, Term::Literal(l)
                        if [xsd::DOUBLE, xsd::FLOAT, xsd::BOOLEAN].contains(&l.datatype()))
            }),
            Self::F15 => !ValueBased::of(input, ours, theirs).is_empty(),
        }
    }

    /// Removes from both closures what this fault makes differ.
    fn filter(self, input: &Input, ours: &mut Closure, theirs: &mut Closure) {
        match self {
            Self::F1 => {
                let aliases = schema_aliases(ours);
                ours.triples
                    .retain(|t| !reflexive(t, &aliases) || theirs.triples.contains(t))
            }
            Self::F2 => {
                for c in [ours, theirs] {
                    c.findings = c
                        .findings
                        .drain()
                        .map(|f| {
                            if f.code == "cls-com" {
                                Finding::new(f.code, f.terms.into_iter().take(2).collect())
                            } else {
                                f
                            }
                        })
                        .collect();
                }
            }
            Self::F3 => ours
                .findings
                .retain(|f| !padded_literal(f) || theirs.findings.contains(f)),
            Self::F4 => {}
            Self::F5 => ours
                .findings
                .retain(|f| !typed_elsewhere(f) || theirs.findings.contains(f)),
            Self::F6 => ours
                .findings
                .retain(|f| !by_list_position(f) || theirs.findings.contains(f)),
            Self::F7 => drop_missed(ours, theirs, python_number),
            Self::F9 => drop_missed(ours, theirs, python_lexical),
            Self::F10 => drop_missed(ours, theirs, strptime_date_time),
            Self::F12 => {
                ours.triples
                    .retain(|t| !rewritten_object(t) || theirs.triples.contains(t));
                drop_missed(ours, theirs, holds_rewritten);
            }
            Self::F8 | Self::F11 | Self::F13 | Self::F14 => {}
            Self::F15 => {
                let explained = ValueBased::of(input, ours, theirs);
                ours.triples.retain(|t| !explained.triples.contains(t));
                ours.findings.retain(|f| !explained.findings.contains(f));
            }
        }
    }
}

/// What F15 explains: the triples and findings of a closure that OWL-RL's
/// closure does not have and that the value-based rules give.
#[derive(Debug, Default)]
struct ValueBased {
    triples: HashSet<oxrdf::Triple>,
    findings: HashSet<Finding>,
}

impl ValueBased {
    fn of(input: &Input, ours: &Closure, theirs: &Closure) -> Self {
        let mut explained = Self::default();
        let missing: Vec<&oxrdf::Triple> = ours.triples.iter().filter(|t| !theirs.triples.contains(*t)).collect();
        let missing_findings: Vec<&Finding> = ours.findings.iter().filter(|f| !theirs.findings.contains(*f)).collect();
        if missing.is_empty() && missing_findings.is_empty() {
            return explained;
        }
        // dt-type2: a term made owl:sameAs a literal has the literal's typings.
        let same_as = "http://www.w3.org/2002/07/owl#sameAs";
        let mut typings: HashSet<(&NamedOrBlankNode, String)> = HashSet::new();
        for t in &ours.triples {
            if t.predicate.as_str() == same_as
                && let Term::Literal(l) = &t.object
                && let Some(value) = turbowl_reference::DataValue::of(l)
            {
                typings.extend(value.datatypes().into_iter().map(|dt| (&t.subject, dt)));
            }
        }
        // dt-eq and dt-diff: what the complete configuration gives without
        // them, a closure every other configuration's is part of.
        let without = match crate::ours::run(input, Switches::complete().off(Switch::LiteralValues)) {
            crate::Outcome::Ran(c) => c,
            other => panic!("TurbOWL without literal-values: {other:?}"),
        };
        for t in missing {
            let typed = t.predicate == rdf::TYPE
                && matches!(&t.object, Term::NamedNode(dt) if typings.contains(&(&t.subject, dt.as_str().to_owned())));
            if typed || !without.triples.contains(t) {
                explained.triples.insert(t.clone());
            }
        }
        explained.findings = missing_findings
            .into_iter()
            .filter(|f| !without.findings.contains(*f))
            .cloned()
            .collect();
        explained
    }

    fn is_empty(&self) -> bool {
        self.triples.is_empty() && self.findings.is_empty()
    }
}

/// Applies every fault of `oracle` that shows on this input to the two
/// closures (`ours` is compared with `theirs`, the oracle's), and returns them.
pub fn apply(oracle: &str, input: &Input, ours: &mut Closure, theirs: &mut Closure) -> Vec<Fault> {
    let shown: Vec<Fault> = Fault::ALL
        .into_iter()
        .filter(|f| f.oracle() == oracle && f.shows(input, ours, theirs))
        .collect();
    for f in &shown {
        f.filter(input, ours, theirs);
    }
    shown
}

/// The fault that explains why `oracle` failed on this input, if one does.
pub fn explains_failure(oracle: &str, input: &Input) -> Option<Fault> {
    (oracle == Fault::F4.oracle() && has_cyclic_list(input)).then_some(Fault::F4)
}

/// The predicates of F1 (`rdfs:subClassOf`, `owl:equivalentClass`,
/// `rdfs:subPropertyOf`, `owl:equivalentProperty`), and every IRI the
/// closure makes `owl:sameAs` one of them.
fn schema_aliases(closure: &Closure) -> HashSet<oxrdf::NamedNode> {
    let predicates: [Term; 4] = [
        rdfs::SUB_CLASS_OF.into_owned().into(),
        oxrdf::NamedNode::new_unchecked("http://www.w3.org/2002/07/owl#equivalentClass").into(),
        rdfs::SUB_PROPERTY_OF.into_owned().into(),
        oxrdf::NamedNode::new_unchecked("http://www.w3.org/2002/07/owl#equivalentProperty").into(),
    ];
    let mut aliases: HashSet<oxrdf::NamedNode> = closure
        .triples
        .iter()
        .filter(|t| t.predicate.as_str() == "http://www.w3.org/2002/07/owl#sameAs" && predicates.contains(&t.object))
        .filter_map(|t| match &t.subject {
            NamedOrBlankNode::NamedNode(n) => Some(n.clone()),
            NamedOrBlankNode::BlankNode(_) => None,
        })
        .collect();
    aliases.extend(predicates.into_iter().filter_map(|p| match p {
        Term::NamedNode(n) => Some(n),
        _ => None,
    }));
    aliases
}

/// `c p c` with `p` one of `aliases` ([`schema_aliases`]).
fn reflexive(t: &oxrdf::Triple, aliases: &HashSet<oxrdf::NamedNode>) -> bool {
    aliases.contains(&t.predicate) && Term::from(t.subject.clone()) == t.object
}

/// A dt-not-type finding about a literal with leading or trailing whitespace.
fn padded_literal(f: &Finding) -> bool {
    f.code == "dt-not-type"
        && matches!(f.terms.first(), Some(Term::Literal(l))
            if l.value() != l.value().trim_matches([' ', '\t', '\n', '\r']))
}

/// A dt-not-type finding whose datatype is not the literal's own.
fn typed_elsewhere(f: &Finding) -> bool {
    match (f.code.as_str(), f.terms.as_slice()) {
        ("dt-not-type", [Term::Literal(l), Term::NamedNode(dt)]) => l.datatype() != dt.as_ref(),
        _ => false,
    }
}

/// A finding of a rule that pairs list positions as F6 says OWL-RL does not.
fn by_list_position(f: &Finding) -> bool {
    matches!(f.code.as_str(), "eq-diff2" | "eq-diff3" | "prp-adp")
}

const OWL_SAME_AS: &str = "http://www.w3.org/2002/07/owl#sameAs";
const OWL_HAS_KEY: &str = "http://www.w3.org/2002/07/owl#hasKey";
const OWL_MAX_CARDINALITY: &str = "http://www.w3.org/2002/07/owl#maxCardinality";
const OWL_MAX_QUALIFIED_CARDINALITY: &str = "http://www.w3.org/2002/07/owl#maxQualifiedCardinality";

/// True when `ours` has a finding `which` selects that `theirs` lacks.
fn missed(ours: &Closure, theirs: &Closure, which: fn(&Finding) -> bool) -> bool {
    ours.findings.iter().any(|f| which(f) && !theirs.findings.contains(f))
}

/// Drops from `ours` the findings `which` selects that `theirs` lacks.
fn drop_missed(ours: &mut Closure, theirs: &Closure, which: fn(&Finding) -> bool) {
    ours.findings.retain(|f| !which(f) || theirs.findings.contains(f));
}

/// True when `theirs` has a dt-not-type finding about a literal of one of
/// `datatypes`, against its own datatype, that `ours` lacks.
fn over_reported(ours: &Closure, theirs: &Closure, datatypes: &[NamedNodeRef<'_>]) -> bool {
    theirs
        .findings
        .iter()
        .any(|f| own_datatype(f).is_some_and(|(_, dt)| datatypes.contains(&dt)) && !ours.findings.contains(f))
}

/// The lexical form and datatype of a dt-not-type finding about a literal
/// and its own datatype (the only check OWL-RL makes, F5).
fn own_datatype(f: &Finding) -> Option<(&str, NamedNodeRef<'_>)> {
    match (f.code.as_str(), f.terms.as_slice()) {
        ("dt-not-type", [Term::Literal(l), Term::NamedNode(dt)]) if l.datatype() == dt.as_ref() => {
            Some((l.value(), dt.as_ref()))
        }
        _ => None,
    }
}

/// The datatypes OWL-RL reads with Python's `int()` or `Decimal()` (F7).
const NUMBERS: [NamedNodeRef<'static>; 16] = [
    xsd::DECIMAL,
    xsd::INTEGER,
    xsd::NON_NEGATIVE_INTEGER,
    xsd::NON_POSITIVE_INTEGER,
    xsd::POSITIVE_INTEGER,
    xsd::NEGATIVE_INTEGER,
    xsd::LONG,
    xsd::INT,
    xsd::SHORT,
    xsd::BYTE,
    xsd::UNSIGNED_LONG,
    xsd::UNSIGNED_INT,
    xsd::UNSIGNED_SHORT,
    xsd::UNSIGNED_BYTE,
    xsd::DOUBLE,
    xsd::FLOAT,
];

/// F7: a dt-not-type finding of a number whose lexical form Python may read
/// and XML Schema's ASCII grammars never do: an underscore, a character
/// outside ASCII (another script's digit, a non-ASCII space), or a decimal
/// NaN or Infinity in any case.
fn python_number(f: &Finding) -> bool {
    let Some((form, dt)) = own_datatype(f) else {
        return false;
    };
    let unsigned = form.strip_prefix(['+', '-']).unwrap_or(form);
    let special = matches!(
        unsigned.to_ascii_lowercase().as_str(),
        "nan" | "snan" | "inf" | "infinity"
    );
    NUMBERS.contains(&dt) && (form.contains('_') || !form.is_ascii() || dt == xsd::DECIMAL && special)
}

/// The datatypes of F9 where OWL-RL rejects forms XML Schema accepts.
const F9_DATATYPES: &[NamedNodeRef<'static>] = &[
    xsd::HEX_BINARY,
    xsd::BASE_64_BINARY,
    xsd::NORMALIZED_STRING,
    xsd::NAME,
    xsd::NC_NAME,
    xsd::NMTOKEN,
    xsd::ANY_URI,
];

/// F9: a dt-not-type finding of a form OWL-RL's reader accepts and XML
/// Schema does not: a boolean in another case, a hexBinary that
/// `int(v, 16)` reads (odd length, sign, `0x`, underscores), a
/// base64Binary with nonzero padding bits, or a name holding a character
/// `\w` matches that is no XML name character.
fn python_lexical(f: &Finding) -> bool {
    let Some((form, dt)) = own_datatype(f) else {
        return false;
    };
    if dt == xsd::BOOLEAN {
        !matches!(form, "true" | "false") && matches!(form.to_lowercase().as_str(), "true" | "false")
    } else if dt == xsd::HEX_BINARY {
        let digits = form.strip_prefix(['+', '-']).unwrap_or(form);
        let digits = digits
            .strip_prefix("0x")
            .or_else(|| digits.strip_prefix("0X"))
            .unwrap_or(digits);
        let hex_binary = form.len().is_multiple_of(2) && form.bytes().all(|b| b.is_ascii_hexdigit());
        !hex_binary && !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_hexdigit() || b == b'_')
    } else if dt == xsd::BASE_64_BINARY {
        nonzero_padding_bits(form)
    } else if [xsd::NAME, xsd::NC_NAME, xsd::NMTOKEN].contains(&dt) {
        form.contains(['ª', '²', '³', 'µ', '¹', 'º', '¼', '½', '¾'])
    } else {
        false
    }
}

/// A base64 form without spaces whose last character before the padding
/// has bits the padding should zero: not a B04char (`[AQgw]`) before `==`,
/// not a B16char (`[AEIMQUYcgkosw048]`) before `=`.
fn nonzero_padding_bits(form: &str) -> bool {
    let data = form.trim_end_matches('=');
    let alphabet = data
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/');
    if !form.len().is_multiple_of(4) || !alphabet {
        return false;
    }
    match (form.len() - data.len(), data.bytes().last()) {
        (2, Some(c)) => !b"AQgw".contains(&c),
        (1, Some(c)) => !b"AEIMQUYcgkosw048".contains(&c),
        _ => false,
    }
}

/// F10: a dt-not-type finding of a dateTime that OWL-RL's reading accepts
/// (a `[+-]dd:dd` zone cut off, a fraction of digits, then
/// `%Y-%m-%dT%H:%M:%S`, which takes one-digit fields and any case) and XML
/// Schema does not: a lowercase `t`, a one-digit month, day, hour, minute or
/// second, or a time zone past 14:00 or with more than 59 minutes.
fn strptime_date_time(f: &Finding) -> bool {
    let Some((form, dt)) = own_datatype(f) else {
        return false;
    };
    if dt != xsd::DATE_TIME && dt != xsd::DATE_TIME_STAMP {
        return false;
    }
    let digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    let zone = |z: &str| {
        let b = z.as_bytes();
        b.len() == 6 && matches!(b[0], b'+' | b'-') && b[3] == b':' && digits(&z[1..3]) && digits(&z[4..])
    };
    let (rest, offset) = match form.strip_suffix('Z') {
        Some(rest) => (rest, None),
        None => match form
            .len()
            .checked_sub(6)
            .and_then(|i| Some((form.get(..i)?, form.get(i..)?)))
        {
            Some((rest, z)) if zone(z) => (rest, Some((&z[1..3], &z[4..]))),
            _ => (form, None),
        },
    };
    let rest = match rest.split_once('.') {
        Some((rest, fraction)) if !fraction.is_empty() && digits(fraction) => rest,
        Some(_) => return false,
        None => rest,
    };
    let Some((date, time)) = rest.split_once(['T', 't']) else {
        return false;
    };
    let (date, time): (Vec<&str>, Vec<&str>) = (date.split('-').collect(), time.split(':').collect());
    let (&[year, month, day], &[hour, minute, second]) = (&date[..], &time[..]) else {
        return false;
    };
    let fields = [month, day, hour, minute, second];
    if year.len() != 4 || !digits(year) || !fields.iter().all(|s| (1..=2).contains(&s.len()) && digits(s)) {
        return false;
    }
    let (hours, minutes): (u32, u32) = offset.map_or((0, 0), |(h, m)| (h.parse().unwrap_or(0), m.parse().unwrap_or(0)));
    rest.contains('t')
        || fields.iter().any(|s| s.len() == 1)
        || hours > 14
        || hours == 14 && minutes > 0
        || minutes > 59
}

/// True when the term is an xsd:token or xsd:normalizedString literal that
/// rdflib builds with another lexical form (rdflib/term.py:711-715,
/// 2284-2299): tabs, line feeds and carriage returns become spaces, and a
/// token is stripped (Python's `str.strip()`) and its runs of spaces
/// collapsed.
fn rdflib_rewrites(term: &Term) -> bool {
    let Term::Literal(l) = term else { return false };
    let (form, dt) = (l.value(), l.datatype());
    if dt != xsd::NORMALIZED_STRING && dt != xsd::TOKEN {
        return false;
    }
    let mut rewritten = form.replace(['\t', '\n', '\r'], " ");
    if dt == xsd::TOKEN {
        let python_space = |c: char| c.is_whitespace() || ('\x1c'..='\x1f').contains(&c);
        let mut collapsed = String::new();
        for c in rewritten.trim_matches(python_space).chars() {
            if !(c == ' ' && collapsed.ends_with(' ')) {
                collapsed.push(c);
            }
        }
        rewritten = collapsed;
    }
    rewritten != form
}

/// A triple whose object is a literal rdflib rewrites (F12); a subject is
/// never a literal.
fn rewritten_object(t: &oxrdf::Triple) -> bool {
    rdflib_rewrites(&t.object)
}

/// A finding naming a literal rdflib rewrites (F12).
fn holds_rewritten(f: &Finding) -> bool {
    f.terms.iter().any(rdflib_rewrites)
}

fn has_predicate(input: &Input, iri: &str) -> bool {
    input.triples().iter().any(|t| t.predicate.as_str() == iri)
}

/// True when following `rdf:rest` from some list cell comes back to it.
fn has_cyclic_list(input: &Input) -> bool {
    let rest: HashMap<NamedOrBlankNode, &Term> = input
        .triples()
        .iter()
        .filter(|t| t.predicate == rdf::REST)
        .map(|t| (t.subject.clone(), &t.object))
        .collect();
    rest.keys().any(|start| {
        let mut seen = HashSet::new();
        let mut cell = start.clone();
        while seen.insert(cell.clone()) {
            let next = match rest.get(&cell) {
                Some(Term::NamedNode(n)) => NamedOrBlankNode::from(n.clone()),
                Some(Term::BlankNode(b)) => NamedOrBlankNode::from(b.clone()),
                _ => return false,
            };
            if next == *start {
                return true;
            }
            cell = next;
        }
        false
    })
}

/// Each fault's test: the faulty oracle disagrees with the W3C rule, and the
/// other oracles (Nemo, HermiT, the W3C tests) agree with it. An oracle that
/// is not installed is skipped (with `CI=true`, the test fails).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::closure::triple;
    use crate::compare::{Relation, compare};
    use crate::w3c::{self, Verdict};
    use crate::{Outcome, hermit, nemo, owlrl};
    use oxrdf::{Literal, NamedNode};

    const PREFIXES: &str = "@prefix ex: <http://example.org/> .
        @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n";

    fn input(name: &str, turtle: &str) -> Input {
        Input::turtle(name, &format!("{PREFIXES}{turtle}"))
    }

    fn ex(local: &str) -> Term {
        NamedNode::new_unchecked(format!("http://example.org/{local}")).into()
    }

    /// Nemo's closure and OWL-RL's outcome, or `None` when one is not installed.
    fn both(input: &Input) -> Option<(Closure, Outcome)> {
        both_with(input, turbowl::Switches::default())
    }

    /// [`both`], with Nemo in the configuration `switches` (OWL-RL has no
    /// switches).
    fn both_with(input: &Input, switches: turbowl::Switches) -> Option<(Closure, Outcome)> {
        let nemo = match nemo::run_with(input, switches) {
            Outcome::Ran(c) => c,
            Outcome::Skipped(why) => {
                println!("skipped: {why}");
                return None;
            }
            other => panic!("Nemo: {other:?}"),
        };
        let owlrl = owlrl::run(input);
        if let Outcome::Skipped(why) = &owlrl {
            println!("skipped: {why}");
            return None;
        }
        Some((nemo, owlrl))
    }

    fn ran(o: Outcome) -> Closure {
        match o {
            Outcome::Ran(c) => c,
            other => panic!("OWL-RL: {other:?}"),
        }
    }

    /// Nemo ⊆ OWL-RL fails without the fault's filter and holds with it.
    fn filter_is_needed_and_enough(fault: Fault, input: &Input, nemo: &Closure, owlrl: &Closure) {
        filter_is_needed_and_enough_for(fault, input, ("Nemo", nemo), owlrl);
    }

    /// `left` ⊆ OWL-RL fails without the fault's filter and holds with it.
    fn filter_is_needed_and_enough_for(fault: Fault, input: &Input, left: (&str, &Closure), owlrl: &Closure) {
        let (name, closure) = left;
        let raw = compare((name, closure), ("OWL-RL", owlrl), Relation::Subset);
        assert!(!raw.holds(), "{fault:?} no longer shows:\n{raw}");
        let (mut ours, mut theirs) = (closure.clone(), owlrl.clone());
        let applied = apply("OWL-RL", input, &mut ours, &mut theirs);
        assert!(applied.contains(&fault), "{fault:?} not detected: {applied:?}");
        let filtered = compare((name, &ours), ("OWL-RL", &theirs), Relation::Subset);
        assert!(filtered.holds(), "{fault:?}: the filter is not enough:\n{filtered}");
    }

    /// A fault that needs no filter: `left` ⊆ OWL-RL holds with the other
    /// faults' filters, and the fault shows.
    fn shows_without_a_filter(fault: Fault, input: &Input, left: (&str, &Closure), owlrl: &Closure) {
        let (name, closure) = left;
        let (mut ours, mut theirs) = (closure.clone(), owlrl.clone());
        let applied = apply("OWL-RL", input, &mut ours, &mut theirs);
        assert!(applied.contains(&fault), "{fault:?} not detected: {applied:?}");
        let filtered = compare((name, &ours), ("OWL-RL", &theirs), Relation::Subset);
        assert!(filtered.holds(), "{fault:?} needs a filter:\n{filtered}");
    }

    /// TurbOWL's closure and the reference evaluator's, in a configuration.
    fn ours_and_reference(input: &Input, switches: turbowl::Switches) -> (Closure, Closure) {
        let ours = match crate::ours::run(input, switches) {
            Outcome::Ran(c) => c,
            other => panic!("TurbOWL: {other:?}"),
        };
        let base: HashSet<_> = input.triples().iter().cloned().collect();
        let evaluation = turbowl_reference::evaluate_within(&base, switches, 10_000).expect("a small closure");
        let findings = evaluation
            .diagnostics
            .iter()
            .map(|(c, t)| Finding::new(c.clone(), t.clone()));
        let reference = Closure::new(evaluation.closure, findings);
        (ours, reference)
    }

    fn dt_not_type(lexical: &str, datatype: NamedNodeRef<'_>) -> Finding {
        let literal = Literal::new_typed_literal(lexical, datatype);
        Finding::new("dt-not-type", vec![literal.into(), datatype.into_owned().into()])
    }

    /// A fault of OWL-RL's dt-not-type on one input holding each literal
    /// (lexical form, datatype, ill-typed under XML Schema 1.1): TurbOWL and
    /// the reference evaluator report exactly the ill-typed ones, OWL-RL
    /// exactly the others; the fault shows on each, and its filter (if the
    /// fault misses a finding) is needed and enough.
    fn dt_not_type_fault(fault: Fault, literals: &[(&str, NamedNodeRef<'_>, bool)]) {
        let turtle: String = literals
            .iter()
            .map(|(lexical, dt, _)| format!("ex:s ex:p {} .\n", Literal::new_typed_literal(*lexical, *dt)))
            .collect();
        let input = input(&format!("{fault:?}"), &turtle);
        let owlrl = match owlrl::run(&input) {
            Outcome::Skipped(why) => return println!("skipped: {why}"),
            o => ran(o),
        };
        let (ours, reference) = ours_and_reference(&input, turbowl::Switches::default());
        let reported = |c: &Closure| -> HashSet<Finding> {
            c.findings.iter().filter(|f| f.code == "dt-not-type").cloned().collect()
        };
        let ill_typed: HashSet<Finding> = literals
            .iter()
            .filter(|(.., ill)| *ill)
            .map(|(lexical, dt, _)| dt_not_type(lexical, *dt))
            .collect();
        let over: HashSet<Finding> = literals
            .iter()
            .filter(|(.., ill)| !*ill)
            .map(|(lexical, dt, _)| dt_not_type(lexical, *dt))
            .collect();
        assert_eq!(reported(&ours), ill_typed, "TurbOWL");
        assert_eq!(reported(&reference), ill_typed, "the reference evaluator");
        assert_eq!(
            reported(&owlrl),
            over,
            "OWL-RL now reads these as XML Schema does: {fault:?} is gone"
        );
        for f in &ill_typed {
            let (mut ours, mut theirs) = (Closure::new([], [f.clone()]), Closure::default());
            assert_eq!(apply("OWL-RL", &input, &mut ours, &mut theirs), [fault], "{f}");
            assert!(ours.findings.is_empty(), "{f}");
        }
        for f in &over {
            let (mut ours, mut theirs) = (Closure::default(), Closure::new([], [f.clone()]));
            assert_eq!(apply("OWL-RL", &input, &mut ours, &mut theirs), [fault], "{f}");
        }
        if ill_typed.is_empty() {
            shows_without_a_filter(fault, &input, ("TurbOWL", &ours), &owlrl);
        } else {
            filter_is_needed_and_enough_for(fault, &input, ("TurbOWL", &ours), &owlrl);
        }
    }

    /// The verdict on the RDF-MT case `id` for a closure of its premise;
    /// `None` when the suite is not fetched.
    fn rdf_mt_verdict(id: &str, closure: impl Fn(&w3c::Case) -> Closure) -> Option<Verdict> {
        let cases = match w3c::rdf_mt() {
            Ok(c) => c,
            Err(o) => {
                println!("skipped: {o:?}");
                return None;
            }
        };
        let case = cases
            .cases
            .iter()
            .find(|c| c.id == id)
            .unwrap_or_else(|| panic!("no W3C case {id}"));
        Some(w3c::judge(case, &closure(case)))
    }

    #[test]
    fn f1_owlrl_scm_sco_skips_reflexive_subclass() {
        let input = input("f1", "ex:A rdfs:subClassOf ex:B . ex:B rdfs:subClassOf ex:A .");
        let Some((nemo, owlrl)) = both(&input) else { return };
        let owlrl = ran(owlrl);
        let a_sub_a =
            triple("<http://example.org/A> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/A>");
        assert!(
            nemo.triples.contains(&a_sub_a),
            "the W3C scm-sco (Nemo) derives A subClassOf A"
        );
        assert!(
            !owlrl.triples.contains(&a_sub_a),
            "OWL-RL now derives A subClassOf A: F1 is gone"
        );
        match hermit::check(&input, &nemo) {
            Ok(s) => assert!(s.not_entailed.is_empty(), "HermiT: {:?}", s.not_entailed),
            Err(o) => println!("HermiT not run: {o:?}"),
        }
        filter_is_needed_and_enough(Fault::F1, &input, &nemo, &owlrl);
    }

    /// F1 through equality: with `rdfs:subClassOf owl:sameAs
    /// owl:equivalentClass`, eq-rep-p copies `A subClassOf A` to
    /// `A equivalentClass A`, which OWL-RL misses with the first.
    #[test]
    fn f1_reaches_the_aliases_of_sub_class_of() {
        let input = input(
            "f1-alias",
            "rdfs:subClassOf owl:sameAs owl:equivalentClass . ex:A rdfs:subClassOf ex:B .",
        );
        let Some((nemo, owlrl)) = both(&input) else { return };
        let owlrl = ran(owlrl);
        let a_eqc_a =
            triple("<http://example.org/A> <http://www.w3.org/2002/07/owl#equivalentClass> <http://example.org/A>");
        assert!(
            nemo.triples.contains(&a_eqc_a),
            "the W3C rules (Nemo) derive A equivalentClass A"
        );
        assert!(
            !owlrl.triples.contains(&a_eqc_a),
            "OWL-RL now derives it: F1 is gone here"
        );
        filter_is_needed_and_enough(Fault::F1, &input, &nemo, &owlrl);
    }

    /// F1 on properties: with `schema` on, scm-spo closes the cycle scm-eqp1
    /// gives reflexively, and scm-eqp2 makes `p owl:equivalentProperty p`;
    /// OWL-RL has neither.
    #[test]
    fn f1_owlrl_scm_spo_skips_reflexive_subproperty() {
        let input = input("f1-property", "ex:p owl:equivalentProperty ex:q .");
        let schema = turbowl::Switches::default().on(turbowl::Switch::Schema);
        let Some((nemo, owlrl)) = both_with(&input, schema) else {
            return;
        };
        let owlrl = ran(owlrl);
        for t in [
            "<http://example.org/p> <http://www.w3.org/2000/01/rdf-schema#subPropertyOf> <http://example.org/p>",
            "<http://example.org/p> <http://www.w3.org/2002/07/owl#equivalentProperty> <http://example.org/p>",
        ] {
            assert!(nemo.triples.contains(&triple(t)), "the W3C rules (Nemo) derive {t}");
            assert!(
                !owlrl.triples.contains(&triple(t)),
                "OWL-RL now derives {t}: F1 is gone here"
            );
        }
        filter_is_needed_and_enough(Fault::F1, &input, &nemo, &owlrl);
    }

    #[test]
    fn f2_owlrl_cls_com_names_the_class() {
        let input = input("f2", "ex:C1 owl:complementOf ex:C2 . ex:x a ex:C1, ex:C2 .");
        let Some((nemo, owlrl)) = both(&input) else { return };
        let owlrl = ran(owlrl);
        let right = Finding::new("cls-com", vec![ex("C1"), ex("C2"), ex("x")]);
        let wrong = Finding::new("cls-com", vec![ex("C1"), ex("C2"), ex("C2")]);
        assert!(
            nemo.findings.contains(&right),
            "the W3C cls-com (Nemo) names the individual"
        );
        assert!(
            owlrl.findings.contains(&wrong),
            "OWL-RL now names the individual: F2 is gone"
        );
        filter_is_needed_and_enough(Fault::F2, &input, &nemo, &owlrl);
    }

    #[test]
    fn f3_owlrl_accepts_padded_integers() {
        let input = input("f3", "ex:s ex:p \" 42\"^^xsd:int .");
        let owlrl = match owlrl::run(&input) {
            Outcome::Skipped(why) => return println!("skipped: {why}"),
            o => ran(o),
        };
        assert!(
            owlrl.findings.is_empty(),
            "OWL-RL now rejects \" 42\"^^xsd:int: F3 is gone"
        );
        // The W3C test says " 3 "^^xsd:int is ill-typed; OWL-RL disagrees.
        if let Some(v) = rdf_mt_verdict("xmlsch-02-whitespace-facet-2", |c| ran(owlrl::run(&c.premise))) {
            assert!(
                matches!(v, Verdict::Fail(_)),
                "OWL-RL now passes the W3C test: F3 is gone"
            );
        }
        let lit = Literal::new_typed_literal(" 42", xsd::INT);
        let mut ours = Closure::new([], [Finding::new("dt-not-type", vec![lit.into(), xsd::INT.into()])]);
        let mut theirs = owlrl;
        assert_eq!(apply("OWL-RL", &input, &mut ours, &mut theirs), [Fault::F3]);
        assert!(ours.findings.is_empty());
    }

    #[test]
    fn f4_owlrl_fails_on_a_cyclic_list() {
        let input = input(
            "f4",
            "ex:p owl:propertyChainAxiom ex:l1 . ex:l1 rdf:first ex:q ; rdf:rest ex:l2 . ex:l2 rdf:first ex:r ; rdf:rest ex:l1 .
             ex:a ex:q ex:b . ex:b ex:r ex:c .",
        );
        let Some((nemo, owlrl)) = both(&input) else { return };
        assert!(
            matches!(owlrl, Outcome::Failed(ref why) if why.contains("recursive")),
            "OWL-RL now runs on a cyclic list: F4 is gone ({owlrl:?})"
        );
        assert!(
            !nemo.has("<http://example.org/a> <http://example.org/p> <http://example.org/c>"),
            "the W3C prp-spo2 (Nemo) derives nothing from a list without rdf:nil"
        );
        assert_eq!(explains_failure("OWL-RL", &input), Some(Fault::F4));
    }

    #[test]
    fn f5_owlrl_ignores_range_typed_literals() {
        let input = input("f5", "ex:p rdfs:range xsd:string . ex:s ex:p \"25\"^^xsd:integer .");
        let owlrl = match owlrl::run(&input) {
            Outcome::Skipped(why) => return println!("skipped: {why}"),
            o => ran(o),
        };
        assert!(
            owlrl.findings.is_empty(),
            "OWL-RL now checks range-typed literals: F5 is gone"
        );
        // The same input is the W3C test datatypes-range-clash, which expects an inconsistency.
        if let Some(v) = rdf_mt_verdict("datatypes-range-clash", |c| ran(owlrl::run(&c.premise))) {
            assert!(
                matches!(v, Verdict::Fail(_)),
                "OWL-RL now passes the W3C test: F5 is gone"
            );
        }
        let lit = Literal::new_typed_literal("25", xsd::INTEGER);
        let finding = Finding::new("dt-not-type", vec![lit.into(), xsd::STRING.into()]);
        if let Some(v) = rdf_mt_verdict("datatypes-range-clash", |_| Closure::new([], [finding.clone()])) {
            assert_eq!(v, Verdict::Pass, "the W3C test accepts the finding the W3C rule gives");
        }
        let mut ours = Closure::new([], [finding]);
        let mut theirs = owlrl;
        assert_eq!(apply("OWL-RL", &input, &mut ours, &mut theirs), [Fault::F5]);
        assert!(ours.findings.is_empty());
    }

    /// Nemo, the reference evaluator and TurbOWL agree on the W3C findings;
    /// OWL-RL reports fewer, and only those the others report.
    #[test]
    fn f6_owlrl_misses_list_positions() {
        let input = input(
            "f6",
            "ex:d a owl:AllDifferent ; owl:members ( ex:a ex:b ex:c ) ; owl:distinctMembers ( ex:c ex:c ) .
             ex:a owl:sameAs ex:b . ex:c owl:sameAs ex:c .
             ex:x a owl:AllDisjointProperties ; owl:members ( ex:p ex:q ) . ex:u ex:p ex:v ; ex:q ex:v .",
        );
        let Some((nemo, owlrl)) = both(&input) else { return };
        let owlrl = ran(owlrl);
        let switches = turbowl::Switches::default();
        let ours = match crate::ours::run(&input, switches) {
            Outcome::Ran(c) => c,
            other => panic!("TurbOWL: {other:?}"),
        };
        let base: HashSet<_> = input.triples().iter().cloned().collect();
        let evaluation = turbowl_reference::evaluate_within(&base, switches, 10_000).expect("a small closure");
        let listed = |findings: &mut dyn Iterator<Item = Finding>| -> HashSet<Finding> {
            findings.filter(by_list_position).collect()
        };
        let reference = listed(
            &mut evaluation
                .diagnostics
                .iter()
                .map(|(c, t)| Finding::new(c.clone(), t.clone())),
        );
        let (nemo_found, ours_found) = (
            listed(&mut nemo.findings.iter().cloned()),
            listed(&mut ours.findings.iter().cloned()),
        );
        let [a, b, c, d, p, q, u, v] = ["a", "b", "c", "d", "p", "q", "u", "v"].map(ex);
        let mut expected: HashSet<Finding> = [
            ("eq-diff3", vec![d.clone(), c.clone(), c.clone()]),
            ("prp-adp", vec![p, q, u, v]),
        ]
        .into_iter()
        .map(|(code, terms)| Finding::new(code, terms))
        .collect();
        for (zi, zj) in [(&a, &a), (&a, &b), (&b, &a), (&b, &b)] {
            expected.insert(Finding::new("eq-diff2", vec![d.clone(), zi.clone(), zj.clone()]));
        }
        assert_eq!(nemo_found, expected, "the W3C rules (Nemo)");
        assert_eq!(reference, expected, "the reference evaluator");
        assert_eq!(ours_found, expected, "TurbOWL");
        let theirs = listed(&mut owlrl.findings.iter().cloned());
        assert!(theirs.is_subset(&expected), "OWL-RL reports {theirs:?}");
        assert!(
            theirs.len() < expected.len(),
            "OWL-RL now reports every position: F6 is gone"
        );
        filter_is_needed_and_enough(Fault::F6, &input, &nemo, &owlrl);
    }

    /// dt-eq, dt-diff and dt-type2 in the complete configuration: Nemo, the
    /// reference evaluator and TurbOWL agree on what the W3C rules give;
    /// OWL-RL has none of it.
    #[test]
    fn f15_owlrl_skips_dt_eq_and_dt_diff_and_narrows_dt_type2() {
        let input = input(
            "f15",
            "ex:s ex:p \"1\"^^xsd:integer . ex:t ex:p \"01\"^^xsd:integer .
             ex:a owl:sameAs \"5\"^^xsd:int .
             ex:d a owl:FunctionalProperty . ex:x ex:d \"3\"^^xsd:integer , \"4\"^^xsd:integer .",
        );
        let switches = turbowl::Switches::complete();
        let nemo = match nemo::run_with(&input, switches) {
            Outcome::Ran(c) => c,
            Outcome::Skipped(why) => return println!("skipped: {why}"),
            other => panic!("Nemo: {other:?}"),
        };
        let owlrl = match owlrl::run(&input) {
            Outcome::Skipped(why) => return println!("skipped: {why}"),
            o => ran(o),
        };
        let ours = match crate::ours::run(&input, switches) {
            Outcome::Ran(c) => c,
            other => panic!("TurbOWL: {other:?}"),
        };
        let base: HashSet<_> = input.triples().iter().cloned().collect();
        let reference = turbowl_reference::evaluate_within(&base, switches, 100_000).expect("a small closure");
        let integer = |lexical: &str| Term::from(Literal::new_typed_literal(lexical, xsd::INTEGER));
        let eq_diff1 = Finding::new("eq-diff1", vec![integer("3"), integer("4")]);
        for t in [
            // dt-eq, then eq-rep-o.
            "<http://example.org/s> <http://example.org/p> \"01\"^^<http://www.w3.org/2001/XMLSchema#integer>",
            // dt-type2, then eq-rep-s.
            "<http://example.org/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2001/XMLSchema#byte>",
        ] {
            let t = triple(t);
            assert!(nemo.triples.contains(&t), "the W3C rules (Nemo) derive {t}");
            assert!(reference.closure.contains(&t), "the reference evaluator derives {t}");
            assert!(ours.triples.contains(&t), "TurbOWL derives {t}");
            assert!(!owlrl.triples.contains(&t), "OWL-RL now derives {t}: F15 is gone here");
        }
        // dt-diff with eq-diff1.
        assert!(
            nemo.findings.contains(&eq_diff1),
            "the W3C rules (Nemo) report {eq_diff1}"
        );
        assert!(
            reference
                .diagnostics
                .contains(&(eq_diff1.code.clone(), eq_diff1.terms.clone())),
            "the reference evaluator reports {eq_diff1}"
        );
        assert!(ours.findings.contains(&eq_diff1), "TurbOWL reports {eq_diff1}");
        assert!(
            !owlrl.findings.iter().any(|f| f.code == "eq-diff1"),
            "OWL-RL now reports eq-diff1: F15 is gone here"
        );
        filter_is_needed_and_enough(Fault::F15, &input, &nemo, &owlrl);
    }

    #[test]
    fn f7_owlrl_reads_numbers_with_python() {
        dt_not_type_fault(
            Fault::F7,
            &[
                ("\u{663}", xsd::INTEGER, true),
                ("1_000", xsd::DECIMAL, true),
                ("NaN", xsd::DECIMAL, true),
                ("1_0", xsd::INT, true),
                ("1_0", xsd::DOUBLE, true),
            ],
        );
    }

    #[test]
    fn f8_owlrl_rejects_double_and_float_values() {
        dt_not_type_fault(
            Fault::F8,
            &[
                ("0", xsd::DOUBLE, false),
                ("-0", xsd::DOUBLE, false),
                ("INF", xsd::DOUBLE, false),
                ("NaN", xsd::DOUBLE, false),
                ("1E400", xsd::DOUBLE, false),
                ("1e-400", xsd::DOUBLE, false),
                ("1E40", xsd::FLOAT, false),
                ("NaN", xsd::FLOAT, false),
            ],
        );
    }

    #[test]
    fn f9_owlrl_misreads_other_lexical_spaces() {
        dt_not_type_fault(
            Fault::F9,
            &[
                ("TRUE", xsd::BOOLEAN, true),
                ("0FB", xsd::HEX_BINARY, true),
                ("0x0F", xsd::HEX_BINARY, true),
                ("QR==", xsd::BASE_64_BINARY, true),
                ("QUJ=", xsd::BASE_64_BINARY, true),
                ("a\u{b2}", xsd::NMTOKEN, true),
                ("a\u{b5}", xsd::NAME, true),
                ("", xsd::HEX_BINARY, false),
                ("Q Q = =", xsd::BASE_64_BINARY, false),
                ("", xsd::BASE_64_BINARY, false),
                ("", xsd::NORMALIZED_STRING, false),
                ("a\u{b7}b", xsd::NAME, false),
                ("a\u{300}", xsd::NMTOKEN, false),
                ("?q", xsd::ANY_URI, false),
                (":x", xsd::ANY_URI, false),
            ],
        );
    }

    #[test]
    fn f10_owlrl_reads_date_times_with_strptime() {
        dt_not_type_fault(
            Fault::F10,
            &[
                ("2000-01-01T00:00:00+15:00", xsd::DATE_TIME, true),
                ("2000-01-01T00:00:00+01:60", xsd::DATE_TIME, true),
                ("2000-1-01T00:00:00", xsd::DATE_TIME, true),
                ("2000-01-01t00:00:00", xsd::DATE_TIME, true),
                ("2000-01-01T00:00:00+15:00", xsd::DATE_TIME_STAMP, true),
                ("0000-01-01T00:00:00", xsd::DATE_TIME, false),
                ("-0044-03-15T12:00:00", xsd::DATE_TIME, false),
                ("12345-01-01T00:00:00", xsd::DATE_TIME, false),
                ("2000-01-01T24:00:00", xsd::DATE_TIME, false),
                ("2000-01-01T23:59:59.1234567", xsd::DATE_TIME, false),
            ],
        );
    }

    #[test]
    fn f11_owlrl_parses_xml_literals_as_documents() {
        dt_not_type_fault(
            Fault::F11,
            &[("plain text", rdf::XML_LITERAL, false), ("", rdf::XML_LITERAL, false)],
        );
    }

    /// rdflib holds other literals than the input's: Nemo keeps the input's
    /// triples, and TurbOWL and the reference evaluator report the
    /// ill-typed forms OWL-RL no longer sees.
    #[test]
    fn f12_rdflib_rewrites_tokens_and_normalized_strings() {
        let input = input(
            "f12",
            "ex:s ex:p \"a  b\"^^xsd:token, \" a\"^^xsd:token, \"a\\tb\"^^xsd:normalizedString, \"a b\"^^xsd:token .",
        );
        let Some((nemo, owlrl)) = both(&input) else { return };
        let owlrl = ran(owlrl);
        let (ours, reference) = ours_and_reference(&input, turbowl::Switches::default());
        let s_p = "<http://example.org/s> <http://example.org/p>";
        let token = "<http://www.w3.org/2001/XMLSchema#token>";
        let normalized = "<http://www.w3.org/2001/XMLSchema#normalizedString>";
        for (written, read) in [
            (format!("\"a  b\"^^{token}"), format!("\"a b\"^^{token}")),
            (format!("\" a\"^^{token}"), format!("\"a\"^^{token}")),
            (format!("\"a\\tb\"^^{normalized}"), format!("\"a b\"^^{normalized}")),
        ] {
            assert!(
                nemo.has(&format!("{s_p} {written}")),
                "the W3C rules (Nemo) keep {written}"
            );
            assert!(
                !owlrl.has(&format!("{s_p} {written}")) && owlrl.has(&format!("{s_p} {read}")),
                "rdflib now keeps {written}: F12 is gone"
            );
        }
        let ill_typed: HashSet<Finding> = [
            ("a  b", xsd::TOKEN),
            (" a", xsd::TOKEN),
            ("a\tb", xsd::NORMALIZED_STRING),
        ]
        .into_iter()
        .map(|(lexical, dt)| dt_not_type(lexical, dt))
        .collect();
        assert_eq!(ours.findings, ill_typed, "TurbOWL");
        assert_eq!(reference.findings, ill_typed, "the reference evaluator");
        assert!(owlrl.findings.is_empty(), "OWL-RL: {:?}", owlrl.findings);
        filter_is_needed_and_enough(Fault::F12, &input, &nemo, &owlrl);
        filter_is_needed_and_enough_for(Fault::F12, &input, ("TurbOWL", &ours), &owlrl);
    }

    /// A one-property key: x and z share the key value, y does not.
    #[test]
    fn f13_owlrl_prp_key_skips_the_last_property() {
        let input = input(
            "f13",
            "ex:C owl:hasKey ( ex:p ) . ex:x a ex:C ; ex:p ex:v . ex:y a ex:C ; ex:p ex:w . ex:z a ex:C ; ex:p ex:v .",
        );
        let owlrl = match owlrl::run(&input) {
            Outcome::Skipped(why) => return println!("skipped: {why}"),
            o => ran(o),
        };
        let nemo = match nemo::run_with(&input, turbowl::Switches::complete()) {
            Outcome::Ran(c) => c,
            Outcome::Skipped(why) => return println!("skipped: {why}"),
            other => panic!("Nemo: {other:?}"),
        };
        let (ours, reference) = ours_and_reference(&input, turbowl::Switches::complete());
        let same = |a: &str, b: &str| {
            format!("<http://example.org/{a}> <http://www.w3.org/2002/07/owl#sameAs> <http://example.org/{b}>")
        };
        for (name, c) in [
            ("Nemo", &nemo),
            ("TurbOWL", &ours),
            ("the reference evaluator", &reference),
        ] {
            assert!(c.has(&same("x", "z")), "{name}: the W3C prp-key gives x sameAs z");
            assert!(!c.has(&same("x", "y")), "{name}: the W3C prp-key gives no x sameAs y");
        }
        assert!(owlrl.has(&same("x", "z")));
        assert!(
            owlrl.has(&same("x", "y")),
            "OWL-RL no longer makes x sameAs y: F13 is gone"
        );
        shows_without_a_filter(Fault::F13, &input, ("Nemo", &nemo), &owlrl);
    }

    /// A double and a boolean cardinality: no W3C rule reads them.
    #[test]
    fn f14_owlrl_cardinalities_compare_python_values() {
        let input = input(
            "f14",
            "ex:r owl:maxCardinality \"1\"^^xsd:double ; owl:onProperty ex:q . ex:u a ex:r ; ex:q ex:a, ex:b .
             ex:r0 owl:maxCardinality \"false\"^^xsd:boolean ; owl:onProperty ex:q . ex:u a ex:r0 .",
        );
        let owlrl = match owlrl::run(&input) {
            Outcome::Skipped(why) => return println!("skipped: {why}"),
            o => ran(o),
        };
        let nemo = match nemo::run_with(&input, turbowl::Switches::complete()) {
            Outcome::Ran(c) => c,
            Outcome::Skipped(why) => return println!("skipped: {why}"),
            other => panic!("Nemo: {other:?}"),
        };
        let (ours, reference) = ours_and_reference(&input, turbowl::Switches::complete());
        let a_same_b = "<http://example.org/a> <http://www.w3.org/2002/07/owl#sameAs> <http://example.org/b>";
        let maxc1 = Finding::new("cls-maxc1", vec![ex("r0"), ex("u"), ex("a")]);
        for (name, c) in [
            ("Nemo", &nemo),
            ("TurbOWL", &ours),
            ("the reference evaluator", &reference),
        ] {
            assert!(!c.has(a_same_b), "{name}: no W3C rule makes a sameAs b");
            assert!(
                !c.findings.iter().any(|f| f.code.starts_with("cls-max")),
                "{name}: {:?}",
                c.findings
            );
        }
        assert!(
            owlrl.has(a_same_b),
            "OWL-RL's cls-maxc2 no longer reads \"1\"^^xsd:double: F14 is gone"
        );
        assert!(
            owlrl.findings.contains(&maxc1),
            "OWL-RL's cls-maxc1 no longer reads \"false\"^^xsd:boolean: F14 is gone"
        );
        shows_without_a_filter(Fault::F14, &input, ("Nemo", &nemo), &owlrl);
    }

    /// The filters of F7, F9, F10 and F12 select no form XML Schema 1.1
    /// accepts, so they never hide a well-typed literal TurbOWL rejects.
    #[test]
    fn the_lexical_filters_select_no_well_typed_form() {
        let well_typed = [
            ("1.5", xsd::DECIMAL),
            ("-0", xsd::INTEGER),
            ("+007", xsd::INT),
            ("NaN", xsd::DOUBLE),
            ("-INF", xsd::FLOAT),
            ("1e5", xsd::DOUBLE),
            ("true", xsd::BOOLEAN),
            ("0", xsd::BOOLEAN),
            ("0FB7", xsd::HEX_BINARY),
            ("", xsd::HEX_BINARY),
            ("QQ==", xsd::BASE_64_BINARY),
            ("QUI=", xsd::BASE_64_BINARY),
            ("QUJD", xsd::BASE_64_BINARY),
            ("\u{e9}t\u{e9}", xsd::NAME),
            ("a\u{b7}b", xsd::NMTOKEN),
            ("2000-01-01T00:00:00+14:00", xsd::DATE_TIME),
            ("2000-01-01T00:00:00-13:59", xsd::DATE_TIME),
            ("2000-12-31T23:59:59.5Z", xsd::DATE_TIME_STAMP),
            ("a b", xsd::TOKEN),
            ("", xsd::TOKEN),
            (" a  b ", xsd::NORMALIZED_STRING),
        ];
        for (lexical, dt) in well_typed {
            let f = dt_not_type(lexical, dt);
            assert!(
                !python_number(&f) && !python_lexical(&f) && !strptime_date_time(&f) && !holds_rewritten(&f),
                "{f}"
            );
        }
    }

    #[test]
    fn finds_cyclic_lists() {
        let cyclic = input("c", "ex:l1 rdf:rest ex:l2 . ex:l2 rdf:rest ex:l1 .");
        assert!(has_cyclic_list(&cyclic));
        let shared = input(
            "s",
            "ex:I owl:intersectionOf _:l1 . _:l1 rdf:first ex:A ; rdf:rest _:l2 . _:l2 rdf:first ex:B ; rdf:rest rdf:nil .
             ex:J owl:intersectionOf _:l2 .",
        );
        assert!(!has_cyclic_list(&shared));
        assert_eq!(explains_failure("OWL-RL", &shared), None);
    }
}
