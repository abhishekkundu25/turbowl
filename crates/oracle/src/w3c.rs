//! The W3C tests, fetched into `data/w3c/` by `scripts/fetch-data.sh`:
//!
//! - the OWL 2 conformance tests in the RL profile (`owl2/profile-RL.rdf`,
//!   the owl.semanticweb.org export): premise and conclusion ontologies in
//!   RDF/XML, embedded as literals;
//! - the RDF 1.1 Semantics entailment tests (`rdf-tests/rdf-mt/`), from
//!   w3c/rdf-tests.
//!
//! [`judge`] checks a closure of a case's premise against what the case
//! expects. A conclusion is entailed when the closure holds every one of its
//! triples, its blank nodes matched to any terms (simple entailment).

use crate::{Closure, Input, Outcome, data_dir, fetched_version};
use oxrdf::vocab::{rdf, rdfs};
use oxrdf::{BlankNode, NamedNode, NamedNodeRef, NamedOrBlankNode, Term, Triple};
use oxrdfxml::RdfXmlParser;
use oxttl::TurtleParser;
use std::collections::HashMap;
use turbowl::{Format, Switches};

const TEST: &str = "http://www.w3.org/2007/OWL/testOntology#";
const MF: &str = "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#";
const OWL: &str = "http://www.w3.org/2002/07/owl#";
const RDF_MT_BASE: &str = "https://w3c.github.io/rdf-tests/rdf/rdf11/rdf-mt/";

/// Which suite a case is from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Suite {
    Owl2Rl,
    RdfMt,
}

/// What a case expects of the closure of its premise.
#[derive(Debug, Clone)]
pub enum Expect {
    /// The closure entails this graph.
    Entails(Input),
    /// The closure does not entail this graph.
    NotEntails(Input),
    /// The premise is inconsistent: some finding is reported.
    Inconsistent,
    /// The premise is consistent: no inconsistency finding is reported.
    Consistent,
}

/// One test case.
#[derive(Debug, Clone)]
pub struct Case {
    pub suite: Suite,
    /// The W3C identifier (`New-Feature-ObjectPropertyChain-BJP-004`, `rdfs-entailment-test001`).
    pub id: String,
    /// Approved by the working group (`test:Approved` or `rdft:Approved`).
    pub approved: bool,
    /// The entailment regime (`RDF-BASED` and/or `DIRECT` for OWL 2; `RDFS`, `RDF` or `simple` for RDF-MT).
    pub regime: String,
    pub premise: Input,
    pub expect: Expect,
    /// RDF-MT only: the datatypes the case recognises (`mf:recognizedDatatypes`).
    /// Datatype findings about any other datatype do not count.
    pub recognized: Option<Vec<NamedNode>>,
}

/// The loaded cases, and those that cannot run here with the reason.
#[derive(Debug, Clone, Default)]
pub struct Cases {
    pub cases: Vec<Case>,
    pub unrunnable: Vec<(String, String)>,
}

/// The OWL 2 conformance tests in the RL profile. `Err` (skipped) when the data is missing.
pub fn owl2_rl() -> Result<Cases, Outcome> {
    let dir = data_dir().join("w3c/owl2");
    let path = dir.join("profile-RL.rdf");
    if fetched_version(&dir).is_none() || !path.exists() {
        return Err(Outcome::skipped(
            "W3C OWL 2 RL tests",
            "data/w3c/owl2 is missing: run scripts/fetch-data.sh",
        ));
    }
    let text = std::fs::read_to_string(&path).map_err(|e| Outcome::Failed(e.to_string()))?;
    // The export declares `<!ENTITY rdf 'http://...'>`; oxrdfxml reads only
    // double-quoted entity values.
    let text: String = text
        .lines()
        .map(|l| {
            if l.trim_start().starts_with("<!ENTITY") {
                l.replace('\'', "\"")
            } else {
                l.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let triples: Vec<Triple> = RdfXmlParser::new()
        .for_slice(&text)
        .collect::<Result<_, _>>()
        .map_err(|e| Outcome::Failed(format!("profile-RL.rdf: {e}")))?;
    let graph = Index::new(&triples);
    let t = |local: &str| NamedNode::new_unchecked(format!("{TEST}{local}"));
    let mut out = Cases::default();
    for subject in graph.subjects(rdf::TYPE, &t("TestCase").into()) {
        let id = graph.literal(subject, &t("identifier")).unwrap_or_default();
        let approved = graph.objects(subject, &t("status")).any(|s| *s == t("Approved").into());
        let regime = graph
            .objects(subject, &t("semantics"))
            .filter_map(|s| match s {
                Term::NamedNode(n) => n.as_str().strip_prefix(TEST).map(str::to_owned),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("+");
        let doc = |local: &str| graph.literal(subject, &t(local));
        let Some(premise_text) = doc("rdfXmlPremiseOntology") else {
            let reason = if doc("fsPremiseOntology").is_some() {
                "premise in functional syntax only"
            } else {
                "no premise"
            };
            out.unrunnable.push((id, reason.to_owned()));
            continue;
        };
        let premise = match rdf_xml(&format!("{id} premise"), &premise_text) {
            Ok(p) => p,
            Err(e) => {
                out.unrunnable.push((id, e));
                continue;
            }
        };
        let types: Vec<&Term> = graph.objects(subject, &rdf::TYPE.into()).collect();
        let has = |local: &str| types.iter().any(|ty| **ty == t(local).into());
        let mut expects = Vec::new();
        if has("InconsistencyTest") {
            expects.push(Ok(Expect::Inconsistent));
        }
        if has("ConsistencyTest") {
            expects.push(Ok(Expect::Consistent));
        }
        if has("PositiveEntailmentTest") {
            expects.push(match doc("rdfXmlConclusionOntology") {
                Some(c) => rdf_xml(&format!("{id} conclusion"), &c).map(|c| Expect::Entails(conclusion(c))),
                None => Err("conclusion in functional syntax only".to_owned()),
            });
        }
        if has("NegativeEntailmentTest") {
            expects.push(match doc("rdfXmlNonConclusionOntology") {
                Some(c) => rdf_xml(&format!("{id} non-conclusion"), &c).map(|c| Expect::NotEntails(conclusion(c))),
                None => Err("non-conclusion in functional syntax only".to_owned()),
            });
        }
        for expect in expects {
            match expect {
                Ok(expect) => out.cases.push(Case {
                    suite: Suite::Owl2Rl,
                    id: id.clone(),
                    approved,
                    regime: regime.clone(),
                    premise: premise.clone(),
                    expect,
                    recognized: None,
                }),
                Err(e) => out.unrunnable.push((id.clone(), e)),
            }
        }
    }
    out.cases.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

/// The RDF 1.1 Semantics entailment tests. `Err` (skipped) when the data is missing.
pub fn rdf_mt() -> Result<Cases, Outcome> {
    let dir = data_dir().join("w3c/rdf-tests");
    let root = dir.join("rdf-mt");
    if fetched_version(&dir).is_none() || !root.join("manifest.ttl").exists() {
        return Err(Outcome::skipped(
            "W3C RDF-MT tests",
            "data/w3c/rdf-tests is missing: run scripts/fetch-data.sh",
        ));
    }
    let text = std::fs::read(root.join("manifest.ttl")).map_err(|e| Outcome::Failed(e.to_string()))?;
    let triples: Vec<Triple> = TurtleParser::new()
        .with_base_iri(RDF_MT_BASE)
        .expect("valid base IRI")
        .for_slice(&text)
        .collect::<Result<_, _>>()
        .map_err(|e| Outcome::Failed(format!("rdf-mt/manifest.ttl: {e}")))?;
    let graph = Index::new(&triples);
    let mf = |local: &str| NamedNode::new_unchecked(format!("{MF}{local}"));
    let approved = NamedNode::new_unchecked("http://www.w3.org/ns/rdftest#Approved");
    let mut out = Cases::default();
    let load = |iri: &Term| -> Result<Input, String> {
        let Term::NamedNode(n) = iri else {
            return Err(format!("{iri} is not a file"));
        };
        let rel = n
            .as_str()
            .strip_prefix(RDF_MT_BASE)
            .ok_or_else(|| format!("{n} is outside the suite"))?;
        let text = std::fs::read_to_string(root.join(rel)).map_err(|e| format!("{rel}: {e}"))?;
        let format = if rel.ends_with(".nt") {
            Format::NTriples
        } else {
            Format::Turtle
        };
        Input::new(rel).with_document(rel, &text, format)
    };
    for (kind, positive) in [("PositiveEntailmentTest", true), ("NegativeEntailmentTest", false)] {
        for subject in graph.subjects(rdf::TYPE, &mf(kind).into()) {
            let id = graph.literal(subject, &mf("name")).unwrap_or_default();
            let regime = graph.literal(subject, &mf("entailmentRegime")).unwrap_or_default();
            let is_approved = graph
                .objects(
                    subject,
                    &NamedNode::new_unchecked("http://www.w3.org/ns/rdftest#approval"),
                )
                .any(|a| *a == approved.clone().into());
            let recognized = graph.objects(subject, &mf("recognizedDatatypes")).next().map(|head| {
                graph
                    .list(head)
                    .into_iter()
                    .filter_map(|t| match t {
                        Term::NamedNode(n) => Some(n.clone()),
                        _ => None,
                    })
                    .collect()
            });
            let premise = match graph.objects(subject, &mf("action")).next().map(load) {
                Some(Ok(p)) => p,
                Some(Err(e)) => {
                    out.unrunnable.push((id, e));
                    continue;
                }
                None => {
                    out.unrunnable.push((id, "no action".to_owned()));
                    continue;
                }
            };
            let expect = match graph.objects(subject, &mf("result")).next() {
                Some(Term::Literal(l)) if l.value() == "false" => {
                    if positive {
                        Ok(Expect::Inconsistent)
                    } else {
                        Ok(Expect::Consistent)
                    }
                }
                Some(r) => load(r).map(|c| {
                    if positive {
                        Expect::Entails(c)
                    } else {
                        Expect::NotEntails(c)
                    }
                }),
                None => Err("no result".to_owned()),
            };
            match expect {
                Ok(expect) => out.cases.push(Case {
                    suite: Suite::RdfMt,
                    id,
                    approved: is_approved,
                    regime,
                    premise,
                    expect,
                    recognized,
                }),
                Err(e) => out.unrunnable.push((id, e)),
            }
        }
    }
    out.cases.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

/// How a closure fared on a case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    /// Why it failed (the conclusion triples not matched, or the findings).
    Fail(String),
}

/// Judges a closure of the case's premise.
pub fn judge(case: &Case, closure: &Closure) -> Verdict {
    // Every finding is an inconsistency: OWL 2 RL's `false` rules, and
    // ill-typed literals (D-inconsistent) of a recognised datatype.
    let recognised = |f: &&crate::Finding| {
        if f.code != "dt-not-type" {
            return true;
        }
        let datatype = f.terms.get(1).and_then(|t| match t {
            Term::NamedNode(n) => Some(n.clone()),
            _ => None,
        });
        match (&case.recognized, datatype) {
            (Some(known), Some(d)) => known.contains(&d),
            _ => true,
        }
    };
    let inconsistent = closure.findings.iter().any(|f| recognised(&f));
    match &case.expect {
        Expect::Entails(c) => match entails(closure, c) {
            Ok(()) => Verdict::Pass,
            Err(missing) => Verdict::Fail(format!("not entailed: {}", missing.join(" "))),
        },
        Expect::NotEntails(c) => match entails(closure, c) {
            Ok(()) => Verdict::Fail("entails the non-conclusion".to_owned()),
            Err(_) => Verdict::Pass,
        },
        Expect::Inconsistent if inconsistent => Verdict::Pass,
        Expect::Inconsistent => Verdict::Fail("no inconsistency reported".to_owned()),
        Expect::Consistent if !inconsistent => Verdict::Pass,
        Expect::Consistent => Verdict::Fail(format!("inconsistency reported: {}", closure.finding_summary())),
    }
}

/// Where the vocabulary of an entry of [`LIKELY_RULES`] shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shows {
    /// An IRI of the premise, as a predicate or an object.
    Premise,
    /// A predicate of the conclusion.
    Conclusion,
    /// The datatype of a literal of the premise. These rules are listed
    /// only when no other entry shows: most inputs have literals.
    Literal,
}

/// The map from vocabulary to the rules that read or derive it: where it
/// shows, a part of the IRI, and the rules. Every rule of the spec's switch
/// table (`docs/specs/v2-1-scoreboard-and-rules.md`, "The switches") is in
/// it (`tests/programs.rs` checks this).
pub const LIKELY_RULES: &[(Shows, &str, &[&str])] = &[
    (Shows::Premise, "rdf-schema#subClassOf", &["cax-sco", "scm-sco"]),
    (
        Shows::Premise,
        "owl#equivalentClass",
        &["cax-eqc1", "cax-eqc2", "scm-eqc1"],
    ),
    (Shows::Premise, "owl#Thing", &["cls-thing", "cls-svf2"]),
    (Shows::Premise, "owl#Nothing", &["cls-nothing1", "cls-nothing2"]),
    (
        Shows::Premise,
        "owl#intersectionOf",
        &["cls-int1", "cls-int2", "scm-int"],
    ),
    (Shows::Premise, "owl#unionOf", &["cls-uni", "scm-uni"]),
    (
        Shows::Premise,
        "owl#someValuesFrom",
        &["cls-svf1", "cls-svf2", "scm-svf1", "scm-svf2"],
    ),
    (
        Shows::Premise,
        "owl#allValuesFrom",
        &["cls-avf", "scm-avf1", "scm-avf2"],
    ),
    (Shows::Premise, "owl#hasValue", &["cls-hv1", "cls-hv2", "scm-hv"]),
    (Shows::Premise, "owl#oneOf", &["cls-oo"]),
    (
        Shows::Premise,
        "rdf-schema#domain",
        &["prp-dom", "scm-dom1", "scm-dom2"],
    ),
    (Shows::Premise, "rdf-schema#range", &["prp-rng", "scm-rng1", "scm-rng2"]),
    (Shows::Premise, "owl#FunctionalProperty", &["prp-fp"]),
    (Shows::Premise, "owl#InverseFunctionalProperty", &["prp-ifp"]),
    (Shows::Premise, "owl#SymmetricProperty", &["prp-symp"]),
    (Shows::Premise, "owl#TransitiveProperty", &["prp-trp"]),
    (Shows::Premise, "rdf-schema#subPropertyOf", &["prp-spo1", "scm-spo"]),
    (
        Shows::Premise,
        "owl#equivalentProperty",
        &["prp-eqp1", "prp-eqp2", "scm-eqp1", "scm-eqp2"],
    ),
    (Shows::Premise, "owl#inverseOf", &["prp-inv1", "prp-inv2"]),
    (Shows::Premise, "owl#propertyChainAxiom", &["prp-spo2"]),
    (
        Shows::Premise,
        "owl#sameAs",
        &["eq-sym", "eq-trans", "eq-rep-s", "eq-rep-p", "eq-rep-o"],
    ),
    (Shows::Premise, "owl#disjointWith", &["cax-dw"]),
    (Shows::Premise, "owl#AllDisjointClasses", &["cax-adc"]),
    (Shows::Premise, "owl#complementOf", &["cls-com"]),
    (Shows::Premise, "owl#maxCardinality", &["cls-maxc1", "cls-maxc2"]),
    (
        Shows::Premise,
        "owl#maxQualifiedCardinality",
        &["cls-maxqc1", "cls-maxqc2", "cls-maxqc3", "cls-maxqc4"],
    ),
    (Shows::Premise, "owl#IrreflexiveProperty", &["prp-irp"]),
    (Shows::Premise, "owl#AsymmetricProperty", &["prp-asyp"]),
    (Shows::Premise, "owl#propertyDisjointWith", &["prp-pdw"]),
    (Shows::Premise, "owl#AllDisjointProperties", &["prp-adp"]),
    (Shows::Premise, "owl#targetIndividual", &["prp-npa1"]),
    (Shows::Premise, "owl#targetValue", &["prp-npa2"]),
    (Shows::Premise, "owl#differentFrom", &["eq-diff1"]),
    (Shows::Premise, "owl#AllDifferent", &["eq-diff2", "eq-diff3"]),
    (Shows::Premise, "owl#hasKey", &["prp-key"]),
    (Shows::Premise, "owl#Class", &["scm-cls"]),
    (Shows::Premise, "owl#ObjectProperty", &["scm-op"]),
    (Shows::Premise, "owl#DatatypeProperty", &["scm-dp"]),
    (Shows::Premise, "owl#AnnotationProperty", &["prp-ap"]),
    (Shows::Premise, "rdf-schema#label", &["prp-ap"]),
    (Shows::Premise, "rdf-schema#comment", &["prp-ap"]),
    (
        Shows::Conclusion,
        "rdf-schema#subPropertyOf",
        &["scm-spo", "scm-eqp1", "scm-op", "scm-dp"],
    ),
    (Shows::Conclusion, "owl#equivalentProperty", &["scm-eqp2"]),
    (Shows::Conclusion, "owl#equivalentClass", &["scm-eqc2"]),
    (Shows::Conclusion, "rdf-schema#domain", &["scm-dom1", "scm-dom2"]),
    (Shows::Conclusion, "rdf-schema#range", &["scm-rng1", "scm-rng2"]),
    (
        Shows::Conclusion,
        "rdf-schema#subClassOf",
        &[
            "scm-cls", "scm-int", "scm-uni", "scm-svf1", "scm-svf2", "scm-avf1", "scm-avf2", "scm-hv",
        ],
    ),
    (Shows::Conclusion, "owl#sameAs", &["eq-ref"]),
    (
        Shows::Literal,
        "XMLSchema#",
        &["dt-type1", "dt-type2", "dt-eq", "dt-diff", "dt-not-type"],
    ),
    (
        Shows::Literal,
        "rdf-syntax-ns#",
        &["dt-type1", "dt-type2", "dt-eq", "dt-diff", "dt-not-type"],
    ),
];

/// The rules a case likely needs that the program of the configuration
/// does not hold ([`crate::programs::rules`]), read from the vocabulary of
/// its premise and its conclusion with [`LIKELY_RULES`] (a report for the
/// cases Nemo fails, not a proof). Empty when no such vocabulary shows.
pub fn missing_rules(case: &Case, switches: Switches) -> Vec<&'static str> {
    let held = crate::programs::rules(switches);
    let premise: Vec<String> = case
        .premise
        .triples()
        .iter()
        .flat_map(|t| [Term::from(t.predicate.clone()), t.object.clone()])
        .filter_map(|t| match t {
            Term::NamedNode(n) => Some(n.into_string()),
            _ => None,
        })
        .collect();
    let literals: Vec<String> = case
        .premise
        .triples()
        .iter()
        .filter_map(|t| match &t.object {
            Term::Literal(l) => Some(l.datatype().as_str().to_owned()),
            _ => None,
        })
        .collect();
    let conclusion: Vec<String> = match &case.expect {
        Expect::Entails(c) | Expect::NotEntails(c) => {
            c.triples().iter().map(|t| t.predicate.as_str().to_owned()).collect()
        }
        _ => Vec::new(),
    };
    let likely = |kinds: &[Shows]| -> Vec<&'static str> {
        let mut out = Vec::new();
        for (shows, part, rules) in LIKELY_RULES.iter().filter(|(s, _, _)| kinds.contains(s)) {
            let iris = match shows {
                Shows::Premise => &premise,
                Shows::Conclusion => &conclusion,
                Shows::Literal => &literals,
            };
            if iris.iter().any(|iri| iri.contains(part)) {
                for rule in *rules {
                    if !held.contains(*rule) && !out.contains(rule) {
                        out.push(*rule);
                    }
                }
            }
        }
        out
    };
    let mut out = likely(&[Shows::Premise, Shows::Conclusion]);
    if out.is_empty() {
        out = likely(&[Shows::Literal]);
    }
    let container = |g: &Input| {
        g.triples().iter().any(|t| {
            [
                Term::from(t.subject.clone()),
                t.predicate.clone().into(),
                t.object.clone(),
            ]
            .iter()
            .any(|term| match term {
                Term::NamedNode(n) => {
                    n.as_str().starts_with("http://www.w3.org/1999/02/22-rdf-syntax-ns#_")
                        || n.as_str().ends_with("rdf-schema#ContainerMembershipProperty")
                        || n.as_str().ends_with("rdf-schema#member")
                }
                _ => false,
            })
        })
    };
    let conclusion_container = match &case.expect {
        Expect::Entails(c) | Expect::NotEntails(c) => container(c),
        _ => false,
    };
    if container(&case.premise) || conclusion_container {
        out.push("rdfs12 and RDFS axioms (container membership; not OWL 2 RL rules)");
    }
    out
}

/// `Ok` when the closure holds every triple of the graph, with its blank
/// nodes matched to terms; otherwise the triples that do not match.
pub fn entails(closure: &Closure, graph: &Input) -> Result<(), Vec<String>> {
    let is_open =
        |t: &Triple| matches!(t.subject, NamedOrBlankNode::BlankNode(_)) || matches!(t.object, Term::BlankNode(_));
    let (open, ground): (Vec<&Triple>, Vec<&Triple>) = graph.triples().iter().partition(|t| is_open(t));
    let missing: Vec<String> = ground
        .iter()
        .filter(|t| !closure.triples.contains(**t))
        .map(|t| format!("{t} ."))
        .collect();
    if !missing.is_empty() {
        return Err(missing);
    }
    let mut by_predicate: HashMap<&NamedNode, Vec<&Triple>> = HashMap::new();
    for t in &closure.triples {
        by_predicate.entry(&t.predicate).or_default().push(t);
    }
    if bind(&open, &by_predicate, &mut HashMap::new()) {
        Ok(())
    } else {
        Err(open.iter().map(|t| format!("{t} .")).collect())
    }
}

/// Backtracking search for blank-node bindings that match every pattern.
fn bind(
    patterns: &[&Triple],
    by_predicate: &HashMap<&NamedNode, Vec<&Triple>>,
    binding: &mut HashMap<BlankNode, Term>,
) -> bool {
    let Some((first, rest)) = patterns.split_first() else {
        return true;
    };
    let Some(candidates) = by_predicate.get(&first.predicate) else {
        return false;
    };
    for c in candidates {
        let mut added = Vec::new();
        let ok = [
            (Term::from(first.subject.clone()), Term::from(c.subject.clone())),
            (first.object.clone(), c.object.clone()),
        ]
        .into_iter()
        .all(|(p, v)| match p {
            Term::BlankNode(b) => match binding.get(&b) {
                Some(bound) => *bound == v,
                None => {
                    binding.insert(b.clone(), v);
                    added.push(b);
                    true
                }
            },
            p => p == v,
        });
        if ok && bind(rest, by_predicate, binding) {
            return true;
        }
        for b in added {
            binding.remove(&b);
        }
    }
    false
}

/// One RDF/XML document.
fn rdf_xml(name: &str, text: &str) -> Result<Input, String> {
    Input::new(name).with_document(name, text, Format::RdfXml)
}

/// A conclusion without its ontology header and its declarations (`x a
/// owl:Class` and the like): OWL 2 RL entailment (theorem PR1) holds for
/// conclusions whose declarations the premise already makes, and the RL rules
/// derive no declarations.
fn conclusion(graph: Input) -> Input {
    let owl = |local: &str| NamedNode::new_unchecked(format!("{OWL}{local}"));
    let declarations: Vec<Term> = [
        "Ontology",
        "Class",
        "ObjectProperty",
        "DatatypeProperty",
        "AnnotationProperty",
        "NamedIndividual",
    ]
    .iter()
    .map(|l| owl(l).into())
    .chain([rdfs::DATATYPE.into_owned().into()])
    .collect();
    let headers: Vec<NamedOrBlankNode> = graph
        .triples()
        .iter()
        .filter(|t| t.predicate == rdf::TYPE && t.object == owl("Ontology").into())
        .map(|t| t.subject.clone())
        .collect();
    let kept: Vec<Triple> = graph
        .triples()
        .iter()
        .filter(|t| !headers.contains(&t.subject))
        .filter(|t| !(t.predicate == rdf::TYPE && declarations.contains(&t.object)))
        .cloned()
        .collect();
    let text = crate::input::ntriples(kept.iter());
    Input::new(graph.name.clone())
        .with_document(graph.name.clone(), &text, Format::NTriples)
        .expect("N-Triples written from parsed triples")
}

/// Lookups over a manifest graph.
struct Index<'a> {
    triples: &'a [Triple],
}

impl<'a> Index<'a> {
    fn new(triples: &'a [Triple]) -> Self {
        Self { triples }
    }

    fn subjects(
        &self,
        predicate: NamedNodeRef<'_>,
        object: &'a Term,
    ) -> impl Iterator<Item = &'a NamedOrBlankNode> + '_ {
        let predicate = predicate.into_owned();
        self.triples
            .iter()
            .filter(move |t| t.predicate == predicate && t.object == *object)
            .map(|t| &t.subject)
    }

    fn objects<'b>(
        &'b self,
        subject: &'b NamedOrBlankNode,
        predicate: &'b NamedNode,
    ) -> impl Iterator<Item = &'a Term> + 'b {
        self.triples
            .iter()
            .filter(move |t| t.subject == *subject && t.predicate == *predicate)
            .map(|t| &t.object)
    }

    /// The members of the RDF list whose head is `head`.
    fn list(&self, head: &Term) -> Vec<&'a Term> {
        let (first, rest) = (rdf::FIRST.into_owned(), rdf::REST.into_owned());
        let mut out = Vec::new();
        let mut cell = head.clone();
        while let Term::BlankNode(_) | Term::NamedNode(_) = &cell {
            if cell == rdf::NIL.into() || out.len() > 1000 {
                break;
            }
            let node: NamedOrBlankNode = match &cell {
                Term::NamedNode(n) => n.clone().into(),
                Term::BlankNode(b) => b.clone().into(),
                _ => break,
            };
            out.extend(self.objects(&node, &first).next());
            match self.objects(&node, &rest).next() {
                Some(next) => cell = next.clone(),
                None => break,
            }
        }
        out
    }

    fn literal(&self, subject: &NamedOrBlankNode, predicate: &NamedNode) -> Option<String> {
        self.objects(subject, predicate).find_map(|o| match o {
            Term::Literal(l) => Some(l.value().to_owned()),
            _ => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::closure::triple;

    #[test]
    fn matches_blank_nodes() {
        let closure = Closure::new(
            [
                triple("<http://example.org/a> <http://example.org/p> <http://example.org/b>"),
                triple("<http://example.org/b> <http://example.org/q> <http://example.org/c>"),
            ],
            [],
        );
        let yes = Input::turtle(
            "yes",
            "@prefix ex: <http://example.org/> . ex:a ex:p _:x . _:x ex:q ex:c .",
        );
        let no = Input::turtle(
            "no",
            "@prefix ex: <http://example.org/> . ex:a ex:p _:x . _:x ex:q ex:a .",
        );
        assert!(entails(&closure, &yes).is_ok());
        assert!(entails(&closure, &no).is_err());
    }
}
