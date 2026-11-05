//! The parity checklist's minimal inputs (docs/research/reasonable-parity.md,
//! "Parity checklist"), items 3 to 15 and the documented differences, each
//! with what the closures must and must not hold.
//!
//! Expectations are lines `<mark> <triple or finding>` in a Turtle-like short
//! form (prefixes `ex:`, `rdf:`, `rdfs:`, `owl:`, `xsd:`):
//!
//! | Mark | Meaning |
//! |---|---|
//! | `+` | in every closure, OWL-RL's included |
//! | `+V` | in every closure of the version 1 rule set (`reasonable`, Nemo, turtle-reasoner; both profiles) |
//! | `+R` | in every closure of the `reasonable` profile |
//! | `-` | in no closure of the version 1 rule set |
//! | `-R` | in no closure of the `reasonable` profile |
//! | `-L` | in no closure of the `owl-rl` profile |
//! | `=` | the `reasonable` profile's closure is exactly the `=` lines |
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
    /// The checklist item or documented difference it covers.
    pub item: &'static str,
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

/// Which closures an expectation line applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Every closure, OWL-RL's included.
    All,
    /// The version 1 rule set, both profiles.
    RuleSet,
    /// The `reasonable` profile.
    Reasonable,
    /// The `owl-rl` profile.
    OwlRl,
}

/// What kind of closure an oracle gives, for [`Case::check`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// OWL-RL: the full W3C rule set.
    FullRl,
    /// The version 1 rule set in the `reasonable` profile.
    Reasonable,
    /// The version 1 rule set in the `owl-rl` profile.
    OwlRl,
}

impl Scope {
    fn covers(self, kind: Kind) -> bool {
        match self {
            Scope::All => true,
            Scope::RuleSet => kind != Kind::FullRl,
            Scope::Reasonable => kind == Kind::Reasonable,
            Scope::OwlRl => kind == Kind::OwlRl,
        }
    }
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
            let (want, scope) = match mark {
                "+" => (true, Scope::All),
                "+V" => (true, Scope::RuleSet),
                "+R" => (true, Scope::Reasonable),
                "-" => (false, Scope::RuleSet),
                "-R" => (false, Scope::Reasonable),
                "-L" => (false, Scope::OwlRl),
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
            if scope.covers(kind) && closure.triples.contains(&triple(&expand(rest))) != want {
                failed.push((*line).to_owned());
            }
        }
        if kind == Kind::Reasonable && !exact.is_empty() {
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
        // 3. Empty input.
        Case {
            name: "empty",
            item: "3",
            turtle: "",
            expect: &[
                "= owl:Thing rdf:type owl:Class",
                "= owl:Nothing rdf:type owl:Class",
                "= owl:Thing rdf:type owl:Thing",
                "= owl:Nothing rdf:type owl:Thing",
                "+ owl:Thing rdf:type owl:Class",
                "+ owl:Nothing rdf:type owl:Class",
            ],
        },
        // 4. owl:Thing typing of subjects only.
        Case {
            name: "thing-typing",
            item: "4",
            turtle: "ex:a ex:p ex:b .",
            expect: &[
                "+R ex:a rdf:type owl:Thing",
                "-R ex:b rdf:type owl:Thing",
                "-R ex:p rdf:type owl:Thing",
                "-L ex:a rdf:type owl:Thing",
            ],
        },
        // 6. One case per implemented rule.
        Case {
            name: "eq-sym-trans",
            item: "6, 8",
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
            item: "6",
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
            item: "6",
            turtle: "ex:p rdfs:domain ex:C . ex:a ex:p ex:b .",
            expect: &["+ ex:a rdf:type ex:C", "- ex:b rdf:type ex:C"],
        },
        Case {
            name: "prp-rng",
            item: "6",
            turtle: "ex:p rdfs:range ex:C . ex:a ex:p ex:b . ex:a ex:p \"lit\" .",
            expect: &["+ ex:b rdf:type ex:C", "- ex:a rdf:type ex:C"],
        },
        Case {
            name: "prp-fp",
            item: "6, 8",
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
            item: "6",
            turtle: "ex:p a owl:InverseFunctionalProperty . ex:a ex:p ex:o . ex:b ex:p ex:o .",
            expect: &["+ ex:a owl:sameAs ex:b", "+ ex:b owl:sameAs ex:a"],
        },
        Case {
            name: "prp-symp",
            item: "6",
            turtle: "ex:p a owl:SymmetricProperty . ex:a ex:p ex:b .",
            expect: &["+ ex:b ex:p ex:a"],
        },
        Case {
            name: "prp-trp",
            item: "6",
            turtle: "ex:p a owl:TransitiveProperty . ex:a ex:p ex:b . ex:b ex:p ex:c . ex:c ex:p ex:d .",
            expect: &["+ ex:a ex:p ex:c", "+ ex:a ex:p ex:d", "- ex:d ex:p ex:a"],
        },
        Case {
            name: "prp-spo1",
            item: "6",
            turtle: "ex:p rdfs:subPropertyOf ex:q . ex:q rdfs:subPropertyOf ex:r . ex:a ex:p ex:b .",
            expect: &["+ ex:a ex:q ex:b", "+ ex:a ex:r ex:b", "- ex:a ex:p ex:c"],
        },
        Case {
            name: "prp-spo2",
            item: "6, 9",
            turtle: "ex:p owl:propertyChainAxiom (ex:q ex:r ex:s) . ex:t owl:propertyChainAxiom (ex:q) .
                     ex:a ex:q ex:b . ex:b ex:r ex:c . ex:c ex:s ex:d . ex:b ex:s ex:e .",
            expect: &["+ ex:a ex:p ex:d", "+ ex:a ex:t ex:b", "- ex:a ex:p ex:e", "- ex:b ex:p ex:d"],
        },
        Case {
            name: "prp-eqp",
            item: "6",
            turtle: "ex:p owl:equivalentProperty ex:q . ex:a ex:p ex:b . ex:c ex:q ex:d .",
            expect: &["+ ex:a ex:q ex:b", "+ ex:c ex:p ex:d"],
        },
        Case {
            name: "prp-inv",
            item: "6",
            turtle: "ex:p owl:inverseOf ex:q . ex:a ex:p ex:b . ex:c ex:q ex:d .",
            expect: &["+ ex:b ex:q ex:a", "+ ex:d ex:p ex:c"],
        },
        Case {
            name: "cls-int",
            item: "6",
            turtle: "ex:C owl:intersectionOf (ex:A ex:B) . ex:x a ex:A, ex:B . ex:y a ex:C . ex:z a ex:A .",
            expect: &[
                "+ ex:x rdf:type ex:C",
                "+ ex:y rdf:type ex:A",
                "+ ex:y rdf:type ex:B",
                "- ex:z rdf:type ex:C",
            ],
        },
        Case {
            name: "cls-uni",
            item: "6",
            turtle: "ex:C owl:unionOf (ex:A ex:B) . ex:x a ex:A . ex:y a ex:B .",
            expect: &["+ ex:x rdf:type ex:C", "+ ex:y rdf:type ex:C"],
        },
        Case {
            name: "cls-svf",
            item: "6",
            turtle: "ex:R owl:onProperty ex:p ; owl:someValuesFrom ex:D . ex:T owl:onProperty ex:q ; owl:someValuesFrom owl:Thing .
                     ex:a ex:p ex:b . ex:b a ex:D . ex:c ex:p ex:e . ex:f ex:q ex:g .",
            expect: &["+ ex:a rdf:type ex:R", "- ex:c rdf:type ex:R", "+ ex:f rdf:type ex:T"],
        },
        Case {
            name: "cls-avf",
            item: "6",
            turtle: "ex:R owl:onProperty ex:p ; owl:allValuesFrom ex:D . ex:a a ex:R ; ex:p ex:b . ex:c ex:p ex:e .",
            expect: &["+ ex:b rdf:type ex:D", "- ex:e rdf:type ex:D"],
        },
        Case {
            name: "cls-hv",
            item: "6",
            turtle: "ex:R owl:onProperty ex:p ; owl:hasValue ex:v . ex:a a ex:R . ex:b ex:p ex:v .",
            expect: &["+ ex:a ex:p ex:v", "+ ex:b rdf:type ex:R"],
        },
        Case {
            name: "cax-sco-scm-sco",
            item: "6",
            turtle: "ex:A rdfs:subClassOf ex:B . ex:B rdfs:subClassOf ex:C . ex:x a ex:A .",
            expect: &["+ ex:x rdf:type ex:B", "+ ex:x rdf:type ex:C", "+ ex:A rdfs:subClassOf ex:C"],
        },
        Case {
            name: "cax-eqc-scm-eqc1",
            item: "6",
            turtle: "ex:A owl:equivalentClass ex:B . ex:x a ex:A . ex:y a ex:B .",
            expect: &[
                "+ ex:x rdf:type ex:B",
                "+ ex:y rdf:type ex:A",
                "+ ex:A rdfs:subClassOf ex:B",
                "+ ex:B rdfs:subClassOf ex:A",
            ],
        },
        // 7. Rules version 1 does not implement add nothing.
        Case {
            name: "not-scm-spo",
            item: "7",
            turtle: "ex:p1 rdfs:subPropertyOf ex:p2 . ex:p2 rdfs:subPropertyOf ex:p3 .",
            expect: &["- ex:p1 rdfs:subPropertyOf ex:p3", "+ ex:p1 rdfs:subPropertyOf ex:p2"],
        },
        Case {
            name: "not-scm-eqc2",
            item: "7",
            turtle: "ex:A rdfs:subClassOf ex:B . ex:B rdfs:subClassOf ex:A .",
            expect: &["- ex:A owl:equivalentClass ex:B", "+ ex:A rdfs:subClassOf ex:A"],
        },
        Case {
            name: "not-eq-ref",
            item: "7",
            turtle: "ex:a ex:p ex:b .",
            expect: &["- ex:a owl:sameAs ex:a", "- ex:p owl:sameAs ex:p"],
        },
        // 9. Chains: no transitivity from BJP-004's shape; a cyclic list terminates.
        Case {
            name: "chain-no-transitivity",
            item: "9",
            turtle: "ex:p owl:propertyChainAxiom (ex:p ex:q) . ex:a ex:p ex:b . ex:b ex:q ex:c . ex:c ex:p ex:d .",
            expect: &["+ ex:a ex:p ex:c", "- ex:a ex:p ex:d"],
        },
        Case {
            name: "chain-cyclic-list",
            item: "9",
            turtle: "ex:p owl:propertyChainAxiom ex:l1 . ex:l1 rdf:first ex:q ; rdf:rest ex:l2 . ex:l2 rdf:first ex:r ; rdf:rest ex:l1 .
                     ex:a ex:q ex:b . ex:b ex:r ex:c .",
            expect: &[],
        },
        // 10. Container membership.
        Case {
            name: "container",
            item: "10",
            turtle: "ex:x rdf:_1 ex:y . ex:x rdf:_0 ex:z . ex:x rdf:_a ex:w .",
            expect: &[
                "+V rdf:_1 rdf:type rdf:Property",
                "+V rdf:_1 rdf:type rdfs:ContainerMembershipProperty",
                "+V rdf:_1 rdfs:subPropertyOf rdfs:member",
                "+V ex:x rdfs:member ex:y",
                "- ex:x rdfs:member ex:z",
                "- ex:x rdfs:member ex:w",
            ],
        },
        // 11-13. Inconsistencies.
        Case {
            name: "cax-dw",
            item: "11",
            turtle: "ex:A owl:disjointWith ex:B . ex:i a ex:A, ex:B .",
            expect: &["! OWLRL.CAX_DW ex:i ex:A ex:B"],
        },
        Case {
            name: "prp-pdw",
            item: "12",
            turtle: "ex:p owl:propertyDisjointWith ex:q . ex:x ex:p ex:y ; ex:q ex:y .",
            expect: &["! OWLRL.PRP_PDW ex:p ex:q ex:x ex:y"],
        },
        Case {
            name: "prp-asyp",
            item: "12",
            turtle: "ex:p a owl:AsymmetricProperty . ex:x ex:p ex:y . ex:y ex:p ex:x .",
            expect: &["! OWLRL.PRP_ASYP ex:p ex:x ex:y", "! OWLRL.PRP_ASYP ex:p ex:y ex:x"],
        },
        Case {
            name: "prp-irp",
            item: "12",
            turtle: "ex:p a owl:IrreflexiveProperty . ex:x ex:p ex:x .",
            expect: &["! OWLRL.PRP_IRP ex:p ex:x"],
        },
        Case {
            name: "cls-nothing2",
            item: "12",
            turtle: "ex:x a owl:Nothing .",
            expect: &["! OWLRL.CLS_NOTHING ex:x"],
        },
        Case {
            name: "cls-com",
            item: "13, D2",
            turtle: "ex:C1 owl:complementOf ex:C2 . ex:x a ex:C1, ex:C2 .",
            expect: &["! OWLRL.CLS_COM ex:C1 ex:C2 ex:x"],
        },
        // 14-15. Datatypes.
        Case {
            name: "rdfs-datatype",
            item: "14",
            turtle: "ex:s ex:p \" 42\"^^xsd:int, \"2147483648\"^^xsd:int, \"abc\"^^xsd:integer, \"+5\"^^xsd:integer,
                     \"-7\"^^xsd:int, \"x\"^^ex:unknown, \"<a></b>\"^^rdf:XMLLiteral .",
            expect: &[
                "! RDFS.DATATYPE \" 42\"^^xsd:int",
                "! RDFS.DATATYPE \"2147483648\"^^xsd:int",
                "! RDFS.DATATYPE \"abc\"^^xsd:integer",
                "! RDFS.DATATYPE \"<a></b>\"^^rdf:XMLLiteral",
            ],
        },
        Case {
            name: "rdfs-datatype-range",
            item: "15",
            turtle: "ex:p rdfs:range xsd:integer . ex:s ex:p \"abc\", \"x\"@en, ex:o, \"5\"^^xsd:int .
                     ex:q rdfs:range ex:Unknown . ex:s ex:q \"1\" .",
            expect: &[
                "! RDFS.DATATYPE_RANGE \"abc\" xsd:integer ex:p",
                "! RDFS.DATATYPE_RANGE \"x\"@en xsd:integer ex:p",
                "! RDFS.DATATYPE_RANGE ex:o xsd:integer ex:p",
            ],
        },
        // Documented differences.
        Case {
            name: "d1-ifp-different-objects",
            item: "D1",
            turtle: "ex:p a owl:InverseFunctionalProperty . ex:a ex:p ex:o1 . ex:b ex:p ex:o2 .",
            expect: &["- ex:a owl:sameAs ex:b"],
        },
        Case {
            name: "d3-language-tag",
            item: "D3",
            turtle: "ex:p rdfs:subPropertyOf rdfs:label . ex:a ex:p \"chat\"@fr .",
            expect: &["+ ex:a rdfs:label \"chat\"@fr"],
        },
        Case {
            name: "d5-shared-list-tail",
            item: "D5",
            turtle: "ex:I owl:intersectionOf _:l1 . _:l1 rdf:first ex:A ; rdf:rest _:l2 . _:l2 rdf:first ex:B ; rdf:rest rdf:nil .
                     ex:J owl:intersectionOf _:l2 . ex:x a ex:B .",
            expect: &["+V ex:x rdf:type ex:J", "- ex:x rdf:type ex:I"],
        },
        // Not in the checklist: a class with two intersection lists
        // (reasonable keeps one list per class, parity research section 1.4).
        Case {
            name: "two-intersections",
            item: "1.4",
            turtle: "ex:C owl:intersectionOf (ex:A ex:B) . ex:C owl:intersectionOf (ex:D ex:E) . ex:x a ex:A, ex:B . ex:y a ex:D, ex:E .",
            expect: &["+ ex:x rdf:type ex:C", "+ ex:y rdf:type ex:C"],
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
            let _ = case.check(&Closure::default(), Kind::Reasonable, &[]);
        }
    }
}
