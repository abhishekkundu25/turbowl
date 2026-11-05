//! The minimal inputs: one per rule and per diagnostic of the rule set
//! (`docs/specs/v1.md`, "The rule set of version 1"), and a few inputs that
//! test how lists, equality and literals are read, each with what the
//! closures must and must not hold under the W3C OWL 2 RL rules.
//!
//! Expectations are lines `<mark> <triple or finding>` in a Turtle-like short
//! form (prefixes `ex:`, `rdf:`, `rdfs:`, `owl:`, `xsd:`):
//!
//! | Mark | Meaning |
//! |---|---|
//! | `+` | in every closure, OWL-RL's included |
//! | `+V` | in every closure of the version 1 rule set (Nemo, turtle-reasoner, the reference evaluator) |
//! | `-` | in no closure of the version 1 rule set (OWL-RL implements more rules) |
//! | `=` | a closure of the version 1 rule set is exactly the `=` lines |
//! | `!` | a finding (code, then terms) every oracle covering that code reports; no other finding is reported |

use crate::Input;
use crate::closure::{terms_in, triple};
use crate::{Closure, Finding};
use oxrdf::Triple;

/// One minimal input.
#[derive(Debug, Clone, Copy)]
pub struct Case {
    /// A short name (`prp-dom`).
    pub name: &'static str,
    /// The W3C rules (or the reading of lists and literals) it covers.
    pub covers: &'static str,
    /// The input, in Turtle, without prefixes.
    pub turtle: &'static str,
    pub expect: &'static [&'static str],
}

const PREFIXES: &str = "@prefix ex: <http://example.org/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
";

const NAMESPACES: [(&str, &str); 5] = [
    ("ex:", "http://example.org/"),
    ("rdf:", "http://www.w3.org/1999/02/22-rdf-syntax-ns#"),
    ("rdfs:", "http://www.w3.org/2000/01/rdf-schema#"),
    ("owl:", "http://www.w3.org/2002/07/owl#"),
    ("xsd:", "http://www.w3.org/2001/XMLSchema#"),
];

/// What kind of closure an oracle gives, for [`Case::check`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// OWL-RL: the full W3C rule set, and axiomatic triples.
    FullRl,
    /// The version 1 rule set: Nemo, turtle-reasoner, the reference evaluator.
    RuleSet,
}

impl Case {
    /// The input.
    pub fn input(&self) -> Input {
        Input::turtle(self.name, &format!("{PREFIXES}{}", self.turtle))
    }

    /// Checks a closure of this input against the expectations. `codes`: the
    /// finding codes the oracle covers. Returns the failed expectations.
    pub fn check(&self, closure: &Closure, kind: Kind, codes: &[&str]) -> Vec<String> {
        let mut failed = Vec::new();
        let mut exact: Vec<Triple> = Vec::new();
        let mut findings: Vec<Finding> = Vec::new();
        for line in self.expect {
            let (mark, rest) = line.split_once(' ').unwrap_or((line, ""));
            let (want, all) = match mark {
                "+" => (true, true),
                "+V" => (true, false),
                "-" => (false, false),
                "=" => {
                    exact.push(triple(&expand(rest)));
                    continue;
                }
                "!" => {
                    let (code, terms) = rest.split_once(' ').unwrap_or((rest, ""));
                    findings.push(Finding::new(code, terms_in(&expand(terms))));
                    continue;
                }
                _ => panic!("{}: unknown mark in {line:?}", self.name),
            };
            let applies = all || kind == Kind::RuleSet;
            if applies && closure.triples.contains(&triple(&expand(rest))) != want {
                failed.push((*line).to_owned());
            }
        }
        if kind == Kind::RuleSet && !exact.is_empty() {
            let want: std::collections::HashSet<Triple> = exact.into_iter().collect();
            if want != closure.triples {
                failed.push(format!(
                    "= closure has {} triples, expected exactly {}",
                    closure.triples.len(),
                    want.len()
                ));
            }
        }
        let want: std::collections::HashSet<&Finding> =
            findings.iter().filter(|f| codes.contains(&f.code.as_str())).collect();
        let got: std::collections::HashSet<&Finding> = closure
            .findings
            .iter()
            .filter(|f| codes.contains(&f.code.as_str()))
            .collect();
        for f in want.difference(&got) {
            failed.push(format!("! missing {f}"));
        }
        for f in got.difference(&want) {
            failed.push(format!("! unexpected {f}"));
        }
        failed
    }
}

/// Expands the short prefixes outside quoted strings into full IRIs.
pub fn expand(short: &str) -> String {
    let mut out = String::new();
    let mut quoted = false;
    let mut escaped = false;
    let mut i = 0;
    while i < short.len() {
        let rest = &short[i..];
        let c = rest.chars().next().expect("in bounds");
        if quoted {
            out.push(c);
            match c {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' => quoted = false,
                _ => {}
            }
            i += c.len_utf8();
            continue;
        }
        let boundary = out.is_empty() || out.ends_with([' ', '^', '(']);
        if let Some((prefix, ns)) = NAMESPACES.iter().find(|(p, _)| boundary && rest.starts_with(p)) {
            let local: String = rest[prefix.len()..]
                .chars()
                .take_while(|c| c.is_alphanumeric() || matches!(c, '_' | '-'))
                .collect();
            out.push_str(&format!("<{ns}{local}>"));
            i += prefix.len() + local.len();
            continue;
        }
        if c == '"' {
            quoted = true;
        }
        out.push(c);
        i += c.len_utf8();
    }
    out
}

/// The minimal inputs.
pub fn minimal() -> Vec<Case> {
    vec![
        // cls-thing and cls-nothing1 are the only triples of an empty input.
        Case {
            name: "empty",
            covers: "cls-thing, cls-nothing1",
            turtle: "",
            expect: &[
                "= owl:Thing rdf:type owl:Class",
                "= owl:Nothing rdf:type owl:Class",
                "+ owl:Thing rdf:type owl:Class",
                "+ owl:Nothing rdf:type owl:Class",
            ],
        },
        // No rule types every subject owl:Thing.
        Case {
            name: "no-thing-typing",
            covers: "cls-thing",
            turtle: "ex:a ex:p ex:b .",
            expect: &[
                "= ex:a ex:p ex:b",
                "= owl:Thing rdf:type owl:Class",
                "= owl:Nothing rdf:type owl:Class",
            ],
        },
        Case {
            name: "eq-sym-trans",
            covers: "eq-sym, eq-trans",
            turtle: "ex:x owl:sameAs ex:y . ex:y owl:sameAs ex:z .",
            expect: &[
                "+ ex:y owl:sameAs ex:x",
                "+ ex:x owl:sameAs ex:z",
                "+ ex:z owl:sameAs ex:x",
                "+ ex:x owl:sameAs ex:x",
                "+ ex:z owl:sameAs ex:z",
            ],
        },
        Case {
            name: "eq-rep-s-p-o",
            covers: "eq-rep-s, eq-rep-p, eq-rep-o",
            turtle: "ex:a owl:sameAs ex:b . ex:p owl:sameAs ex:q . ex:o owl:sameAs ex:o2 . ex:a ex:p ex:o .",
            expect: &[
                "+ ex:b ex:p ex:o",
                "+ ex:a ex:q ex:o",
                "+ ex:a ex:p ex:o2",
                "+ ex:b ex:q ex:o2",
            ],
        },
        Case {
            name: "prp-dom",
            covers: "prp-dom",
            turtle: "ex:p rdfs:domain ex:C . ex:a ex:p ex:b .",
            expect: &["+ ex:a rdf:type ex:C", "- ex:b rdf:type ex:C"],
        },
        // A class range with an IRI object and a literal object: typing only,
        // no dt-not-type (ex:C is not a datatype).
        Case {
            name: "prp-rng",
            covers: "prp-rng, dt-not-type",
            turtle: "ex:p rdfs:range ex:C . ex:a ex:p ex:b . ex:a ex:p \"lit\" .",
            expect: &["+ ex:b rdf:type ex:C", "- ex:a rdf:type ex:C"],
        },
        // A literal value: `"v" owl:sameAs ex:o1` has a literal subject and is
        // left out of the closure; its mirror is kept.
        Case {
            name: "prp-fp",
            covers: "prp-fp, eq-sym, eq-trans",
            turtle: "ex:p a owl:FunctionalProperty . ex:a ex:p ex:o1 . ex:a ex:p ex:o2 . ex:a ex:p \"v\" .",
            expect: &[
                "+ ex:o1 owl:sameAs ex:o2",
                "+ ex:o2 owl:sameAs ex:o1",
                "+ ex:o1 owl:sameAs ex:o1",
                "+ ex:o1 owl:sameAs \"v\"",
            ],
        },
        Case {
            name: "prp-ifp",
            covers: "prp-ifp",
            turtle: "ex:p a owl:InverseFunctionalProperty . ex:a ex:p ex:o . ex:b ex:p ex:o .",
            expect: &["+ ex:a owl:sameAs ex:b", "+ ex:b owl:sameAs ex:a"],
        },
        // Different objects: nothing is equal.
        Case {
            name: "prp-ifp-different-objects",
            covers: "prp-ifp",
            turtle: "ex:p a owl:InverseFunctionalProperty . ex:a ex:p ex:o1 . ex:b ex:p ex:o2 .",
            expect: &["- ex:a owl:sameAs ex:b", "- ex:b owl:sameAs ex:a"],
        },
        Case {
            name: "prp-symp",
            covers: "prp-symp",
            turtle: "ex:p a owl:SymmetricProperty . ex:a ex:p ex:b .",
            expect: &["+ ex:b ex:p ex:a"],
        },
        Case {
            name: "prp-trp",
            covers: "prp-trp",
            turtle: "ex:p a owl:TransitiveProperty . ex:a ex:p ex:b . ex:b ex:p ex:c . ex:c ex:p ex:d .",
            expect: &["+ ex:a ex:p ex:c", "+ ex:a ex:p ex:d", "- ex:d ex:p ex:a"],
        },
        Case {
            name: "prp-spo1",
            covers: "prp-spo1",
            turtle: "ex:p rdfs:subPropertyOf ex:q . ex:q rdfs:subPropertyOf ex:r . ex:a ex:p ex:b .",
            expect: &["+ ex:a ex:q ex:b", "+ ex:a ex:r ex:b", "- ex:a ex:p ex:c"],
        },
        Case {
            name: "prp-spo2",
            covers: "prp-spo2",
            turtle: "ex:p owl:propertyChainAxiom (ex:q ex:r ex:s) . ex:t owl:propertyChainAxiom (ex:q) .
                     ex:a ex:q ex:b . ex:b ex:r ex:c . ex:c ex:s ex:d . ex:b ex:s ex:e .",
            expect: &["+ ex:a ex:p ex:d", "+ ex:a ex:t ex:b", "- ex:a ex:p ex:e", "- ex:b ex:p ex:d"],
        },
        // The chain of BJP-004's shape is not transitivity.
        Case {
            name: "chain-no-transitivity",
            covers: "prp-spo2",
            turtle: "ex:p owl:propertyChainAxiom (ex:p ex:q) . ex:a ex:p ex:b . ex:b ex:q ex:c . ex:c ex:p ex:d .",
            expect: &["+ ex:a ex:p ex:c", "- ex:a ex:p ex:d"],
        },
        // A cyclic chain list never reaches rdf:nil: it matches nothing, and
        // the run terminates (OWL-RL fails on it: F4).
        Case {
            name: "chain-cyclic-list",
            covers: "prp-spo2",
            turtle: "ex:p owl:propertyChainAxiom ex:l1 . ex:l1 rdf:first ex:q ; rdf:rest ex:l2 . ex:l2 rdf:first ex:r ; rdf:rest ex:l1 .
                     ex:a ex:q ex:b . ex:b ex:r ex:c .",
            expect: &["- ex:a ex:p ex:c", "- ex:a ex:p ex:b"],
        },
        Case {
            name: "prp-eqp",
            covers: "prp-eqp1, prp-eqp2",
            turtle: "ex:p owl:equivalentProperty ex:q . ex:a ex:p ex:b . ex:c ex:q ex:d .",
            expect: &["+ ex:a ex:q ex:b", "+ ex:c ex:p ex:d"],
        },
        Case {
            name: "prp-inv",
            covers: "prp-inv1, prp-inv2",
            turtle: "ex:p owl:inverseOf ex:q . ex:a ex:p ex:b . ex:c ex:q ex:d .",
            expect: &["+ ex:b ex:q ex:a", "+ ex:d ex:p ex:c"],
        },
        Case {
            name: "cls-int",
            covers: "cls-int1, cls-int2",
            turtle: "ex:C owl:intersectionOf (ex:A ex:B) . ex:x a ex:A, ex:B . ex:y a ex:C . ex:z a ex:A .",
            expect: &[
                "+ ex:x rdf:type ex:C",
                "+ ex:y rdf:type ex:A",
                "+ ex:y rdf:type ex:B",
                "- ex:z rdf:type ex:C",
            ],
        },
        // A class with two intersection lists: both count.
        Case {
            name: "two-intersections",
            covers: "cls-int1, cls-int2",
            turtle: "ex:C owl:intersectionOf (ex:A ex:B) . ex:C owl:intersectionOf (ex:D ex:E) .
                     ex:x a ex:A, ex:B . ex:y a ex:D, ex:E . ex:z a ex:A, ex:E .",
            expect: &[
                "+ ex:x rdf:type ex:C",
                "+ ex:y rdf:type ex:C",
                "+ ex:x rdf:type ex:D",
                "+ ex:y rdf:type ex:A",
                "- ex:z rdf:type ex:C",
            ],
        },
        // Two lists that share a tail: each head reads its own list.
        Case {
            name: "shared-list-tail",
            covers: "cls-int1, lists",
            turtle: "ex:I owl:intersectionOf _:l1 . _:l1 rdf:first ex:A ; rdf:rest _:l2 . _:l2 rdf:first ex:B ; rdf:rest rdf:nil .
                     ex:J owl:intersectionOf _:l2 . ex:x a ex:B .",
            expect: &["+ ex:x rdf:type ex:J", "- ex:x rdf:type ex:I"],
        },
        // List triples derived during the run count: the list's rdf:first
        // comes from prp-spo1, its head from prp-eqp1.
        Case {
            name: "derived-list",
            covers: "cls-int1, cls-uni, lists",
            turtle: "ex:first rdfs:subPropertyOf rdf:first . ex:inter owl:equivalentProperty owl:intersectionOf .
                     ex:C ex:inter _:l . _:l ex:first ex:A ; rdf:rest rdf:nil . ex:x a ex:A .
                     ex:U owl:unionOf _:m . _:m ex:first ex:A ; rdf:rest rdf:nil .",
            expect: &[
                "+V ex:C owl:intersectionOf _:b0",
                "+V ex:x rdf:type ex:C",
                "+V ex:x rdf:type ex:U",
            ],
        },
        Case {
            name: "cls-uni",
            covers: "cls-uni",
            turtle: "ex:C owl:unionOf (ex:A ex:B) . ex:x a ex:A . ex:y a ex:B .",
            expect: &["+ ex:x rdf:type ex:C", "+ ex:y rdf:type ex:C"],
        },
        Case {
            name: "cls-svf",
            covers: "cls-svf1, cls-svf2",
            turtle: "ex:R owl:onProperty ex:p ; owl:someValuesFrom ex:D . ex:T owl:onProperty ex:q ; owl:someValuesFrom owl:Thing .
                     ex:a ex:p ex:b . ex:b a ex:D . ex:c ex:p ex:e . ex:f ex:q ex:g .",
            expect: &["+ ex:a rdf:type ex:R", "- ex:c rdf:type ex:R", "+ ex:f rdf:type ex:T"],
        },
        Case {
            name: "cls-avf",
            covers: "cls-avf",
            turtle: "ex:R owl:onProperty ex:p ; owl:allValuesFrom ex:D . ex:a a ex:R ; ex:p ex:b . ex:c ex:p ex:e .",
            expect: &["+ ex:b rdf:type ex:D", "- ex:e rdf:type ex:D"],
        },
        Case {
            name: "cls-hv",
            covers: "cls-hv1, cls-hv2",
            turtle: "ex:R owl:onProperty ex:p ; owl:hasValue ex:v . ex:a a ex:R . ex:b ex:p ex:v .",
            expect: &["+ ex:a ex:p ex:v", "+ ex:b rdf:type ex:R"],
        },
        Case {
            name: "cax-sco-scm-sco",
            covers: "cax-sco, scm-sco",
            turtle: "ex:A rdfs:subClassOf ex:B . ex:B rdfs:subClassOf ex:C . ex:x a ex:A .",
            expect: &["+ ex:x rdf:type ex:B", "+ ex:x rdf:type ex:C", "+ ex:A rdfs:subClassOf ex:C"],
        },
        Case {
            name: "cax-eqc-scm-eqc1",
            covers: "cax-eqc1, cax-eqc2, scm-eqc1",
            turtle: "ex:A owl:equivalentClass ex:B . ex:x a ex:A . ex:y a ex:B .",
            expect: &[
                "+ ex:x rdf:type ex:B",
                "+ ex:y rdf:type ex:A",
                "+ ex:A rdfs:subClassOf ex:B",
                "+ ex:B rdfs:subClassOf ex:A",
            ],
        },
        // scm-sco through a cycle gives A subClassOf A (OWL-RL does not: F1);
        // scm-eqc2 is not in version 1.
        Case {
            name: "scm-sco-cycle",
            covers: "scm-sco, not scm-eqc2",
            turtle: "ex:A rdfs:subClassOf ex:B . ex:B rdfs:subClassOf ex:A .",
            expect: &["+V ex:A rdfs:subClassOf ex:A", "- ex:A owl:equivalentClass ex:B"],
        },
        // Rules version 1 does not implement add nothing.
        Case {
            name: "not-scm-spo",
            covers: "not scm-spo",
            turtle: "ex:p1 rdfs:subPropertyOf ex:p2 . ex:p2 rdfs:subPropertyOf ex:p3 .",
            expect: &["- ex:p1 rdfs:subPropertyOf ex:p3", "+ ex:p1 rdfs:subPropertyOf ex:p2"],
        },
        Case {
            name: "not-eq-ref",
            covers: "not eq-ref",
            turtle: "ex:a ex:p ex:b .",
            expect: &["- ex:a owl:sameAs ex:a", "- ex:p owl:sameAs ex:p"],
        },
        // No container-membership axioms (not OWL 2 RL rules).
        Case {
            name: "no-container-axioms",
            covers: "not rdfs12",
            turtle: "ex:x rdf:_1 ex:y .",
            expect: &[
                "- rdf:_1 rdf:type rdfs:ContainerMembershipProperty",
                "- rdf:_1 rdfs:subPropertyOf rdfs:member",
                "- ex:x rdfs:member ex:y",
            ],
        },
        // Language tags survive derivation.
        Case {
            name: "language-tag",
            covers: "prp-spo1",
            turtle: "ex:p rdfs:subPropertyOf rdfs:label . ex:a ex:p \"chat\"@fr .",
            expect: &["+ ex:a rdfs:label \"chat\"@fr"],
        },
        // Inconsistencies.
        Case {
            name: "cax-dw",
            covers: "cax-dw",
            turtle: "ex:A owl:disjointWith ex:B . ex:i a ex:A, ex:B .",
            expect: &["! cax-dw ex:i ex:A ex:B"],
        },
        Case {
            name: "prp-pdw",
            covers: "prp-pdw",
            turtle: "ex:p owl:propertyDisjointWith ex:q . ex:x ex:p ex:y ; ex:q ex:y .",
            expect: &["! prp-pdw ex:p ex:q ex:x ex:y"],
        },
        Case {
            name: "prp-asyp",
            covers: "prp-asyp",
            turtle: "ex:p a owl:AsymmetricProperty . ex:x ex:p ex:y . ex:y ex:p ex:x .",
            expect: &["! prp-asyp ex:p ex:x ex:y", "! prp-asyp ex:p ex:y ex:x"],
        },
        Case {
            name: "prp-irp",
            covers: "prp-irp",
            turtle: "ex:p a owl:IrreflexiveProperty . ex:x ex:p ex:x .",
            expect: &["! prp-irp ex:p ex:x"],
        },
        Case {
            name: "cls-nothing2",
            covers: "cls-nothing2",
            turtle: "ex:x a owl:Nothing .",
            expect: &["! cls-nothing2 ex:x"],
        },
        // OWL-RL names ex:C2 for ex:x (F2).
        Case {
            name: "cls-com",
            covers: "cls-com",
            turtle: "ex:C1 owl:complementOf ex:C2 . ex:x a ex:C1, ex:C2 .",
            expect: &["! cls-com ex:C1 ex:C2 ex:x"],
        },
        // A literal against its own datatype (OWL-RL accepts " 42": F3).
        Case {
            name: "dt-not-type",
            covers: "dt-not-type",
            turtle: "ex:s ex:p \" 42\"^^xsd:int, \"2147483648\"^^xsd:int, \"abc\"^^xsd:integer, \"+5\"^^xsd:integer,
                     \"-7\"^^xsd:int, \"x\"^^ex:unknown .",
            expect: &[
                "! dt-not-type \" 42\"^^xsd:int xsd:int",
                "! dt-not-type \"2147483648\"^^xsd:int xsd:int",
                "! dt-not-type \"abc\"^^xsd:integer xsd:integer",
            ],
        },
        // A literal typed by prp-rng (OWL-RL does not check these: F5). An
        // IRI object is not a literal, and "5"^^xsd:int is an xsd:integer.
        Case {
            name: "dt-not-type-range",
            covers: "prp-rng, dt-not-type",
            turtle: "ex:p rdfs:range xsd:integer . ex:s ex:p \"abc\", \"x\"@en, ex:o, \"5\"^^xsd:int .
                     ex:q rdfs:range ex:Unknown . ex:s ex:q \"1\" .",
            expect: &[
                "+ ex:o rdf:type xsd:integer",
                "! dt-not-type \"abc\" xsd:integer",
                "! dt-not-type \"x\"@en xsd:integer",
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_prefixes_outside_strings() {
        assert_eq!(
            expand("ex:a rdfs:label \"ex:b\"^^xsd:string"),
            "<http://example.org/a> <http://www.w3.org/2000/01/rdf-schema#label> \"ex:b\"^^<http://www.w3.org/2001/XMLSchema#string>"
        );
    }

    #[test]
    fn every_case_parses() {
        for case in minimal() {
            let _ = case.input();
            let _ = case.check(&Closure::default(), Kind::RuleSet, &[]);
        }
    }
}
